use super::types::json;
use axum::response::sse;
use axum::response::Sse;
use futures::future::Either;
use futures::stream;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

use axum::response::IntoResponse;
use axum::{extract::Query as axumQuery, Extension};
/// We will invoke the agent to get the answer, we are moving to an agent based work
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::llm_funcs::LlmClient;
use crate::agent::types::Agent;
use crate::agent::types::AgentAction;
use crate::agent::types::CodeSpan;
use crate::agent::types::ConversationMessage;
use crate::application::application::Application;
use crate::indexes::code_snippet::CodeSnippetDocument;
use crate::repo::types::RepoRef;

use super::types::ApiResponse;
use super::types::Result;

// We give a timeout of 1 minute between responses
const TIMEOUT_SECS: u64 = 60;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SearchInformation {
    pub query: String,
    pub reporef: RepoRef,
}

impl ApiResponse for SearchInformation {}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SearchResponse {
    pub query: String,
    pub answer: String,
}

impl ApiResponse for SearchResponse {}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum SearchEvents {
    SearchEvent(),
}

pub async fn search_agent(
    axumQuery(SearchInformation { query, reporef }): axumQuery<SearchInformation>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    let session_id = uuid::Uuid::new_v4();
    let llm_client = Arc::new(LlmClient::codestory_infra());
    let conversation_id = uuid::Uuid::new_v4();
    let sql_db = app.sql.clone();

    // Process the events in parallel here
    let conversation_message_stream = async_stream::try_stream! {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        let (answer_sender, answer_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut answer_receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(answer_receiver);
        let mut conversation_message_stream = tokio_stream::wrappers::ReceiverStream::new(receiver);

        let mut agent = Agent::prepare_for_search(
            app,
            reporef,
            session_id,
            &query,
            llm_client,
            conversation_id,
            sql_db,
            sender,
        );

        let mut action = AgentAction::Query(query);

        // poll from both the streams at the same time, we should probably move
        // this to a common place later on as I can see many other places doing
        // the same thing
        let result = 'outer: loop {

            use futures::future::FutureExt;

            let conversation_message_stream_left = (&mut conversation_message_stream).map(Either::Left);
            // map the agent conversation update stream to right::left
            let agent_conversation_update_stream_right = agent
                .iterate(action, answer_sender.clone())
                .into_stream()
                .map(|answer| Either::Right(Either::Left(answer)));
            // map the agent answer stream to right::right
            let agent_answer_delta_stream_left = (&mut answer_receiver_stream).map(|answer| Either::Right(Either::Right(answer)));

            let timeout = Duration::from_secs(TIMEOUT_SECS);
            let mut next = None;
            for await item in tokio_stream::StreamExt::timeout(
                stream::select(conversation_message_stream_left, stream::select(agent_conversation_update_stream_right, agent_answer_delta_stream_left)),
                timeout,
            ) {
                match item {
                    Ok(Either::Left(conversation_message)) => yield conversation_message,
                    Ok(Either::Right(Either::Left(next_action))) => match next_action {
                        Ok(n) => break next = n,
                        Err(e) => {
                            break 'outer Err(anyhow::anyhow!(e))
                        },
                    },
                    Ok(Either::Right(Either::Right(answer_update))) => {
                        // We are going to send the answer update in the same
                        // way as we send the answer
                        let conversation_message = ConversationMessage::answer_update(session_id, answer_update);
                        yield conversation_message
                    }
                    Err(_) => break 'outer Err(anyhow::anyhow!("timeout")),
                }
            }

            // If we have some elements which are still present in the stream, we
            // return them here so as to not loose things in case the timeout got triggered
            // this is basically draining the stream properly
            while let Some(Some(conversation_message)) = conversation_message_stream.next().now_or_never() {
                yield conversation_message;
            }

            // yield the answer from the answer stream so we can send incremental updates here
            while let Some(Some(answer_update)) = answer_receiver_stream.next().now_or_never() {
                let conversation_message = ConversationMessage::answer_update(session_id, answer_update);
                yield conversation_message
            }

            match next {
                Some(a) => action = a,
                None => break Ok(()),
            }
        };

        result?;
    };

    // TODO(skcd): Re-introduce this again when we have a better way to manage
    // server side events on the client side
    let init_stream = futures::stream::once(async move {
        Ok(sse::Event::default()
            .json_data(json!({
                "session_id": session_id,
            }))
            // This should never happen, so we force an unwrap.
            .expect("failed to serialize initialization object"))
    });

    // We know the stream is unwind safe as it doesn't use synchronization primitives like locks.
    let answer_stream = conversation_message_stream.map(
        |conversation_message: anyhow::Result<ConversationMessage>| {
            if let Err(e) = &conversation_message {
                error!("error in conversation message stream: {}", e);
            }
            sse::Event::default()
                .json_data(conversation_message.expect("should not fail deserialization"))
                .map_err(anyhow::Error::new)
        },
    );

    // TODO(skcd): Re-introduce this again when we have a better way to manage
    // server side events on the client side
    let done_stream = futures::stream::once(async move {
        Ok(sse::Event::default()
            .json_data(json!(
                {"done": "[CODESTORY_DONE]".to_owned(),
                "session_id": session_id,
            }))
            .expect("failed to send done object"))
    });

    let stream = init_stream.chain(answer_stream).chain(done_stream);

    Ok(Sse::new(Box::pin(stream)))
}

// TODO(skcd): Add write files and other things here
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SemanticSearchQuery {
    pub query: String,
    pub reporef: RepoRef,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SemanticSearchResponse {
    session_id: uuid::Uuid,
    query: String,
    code_spans: Vec<CodeSpan>,
}

impl ApiResponse for SemanticSearchResponse {}

pub async fn semantic_search(
    axumQuery(SemanticSearchQuery { query, reporef }): axumQuery<SemanticSearchQuery>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    // The best thing to do here is the following right now:
    // lexical search on the paths of the code
    // and then semantic search on the chunks we have from the file
    // we return at this point, because the latency is too high, and this is
    // okay as it is
    let session_id = uuid::Uuid::new_v4();
    let llm_client = Arc::new(LlmClient::codestory_infra());
    let conversation_id = uuid::Uuid::new_v4();
    let sql_db = app.sql.clone();
    let (sender, _) = tokio::sync::mpsc::channel(100);
    let mut agent = Agent::prepare_for_semantic_search(
        app,
        reporef,
        session_id,
        &query,
        llm_client,
        conversation_id,
        sql_db,
        sender,
    );
    let code_spans = agent
        .semantic_search()
        .await
        .expect("semantic_search to not fail");
    Ok(json(SemanticSearchResponse {
        session_id,
        query,
        code_spans,
    }))
}

// Here we are experimenting with lexical search:
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct SearchQuery {
    query: String,
    repo: RepoRef,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct SearchResponseForLexicalSearch {
    code_documents: Vec<CodeSnippetDocument>,
    repo: RepoRef,
}

impl ApiResponse for SearchResponseForLexicalSearch {}

impl ApiResponse for SearchQuery {}

pub async fn lexical_search(
    axumQuery(SemanticSearchQuery { query, reporef }): axumQuery<SemanticSearchQuery>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    let documents = app
        .indexes
        .code_snippet
        .lexical_search(&reporef, &query, 10)
        .await
        .expect("lexical search to not fail");
    Ok(json(SearchResponseForLexicalSearch {
        code_documents: documents,
        repo: reporef,
    }))
}

// Here we are going to provide a hybrid search index which combines both the
// lexical and the semantic search together
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HybridSearchQuery {
    query: String,
    repo: RepoRef,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HybridSearchResponse {
    session_id: uuid::Uuid,
    query: String,
    code_spans: Vec<CodeSpan>,
}

impl ApiResponse for HybridSearchResponse {}

/// What's hybrid search? Hybrid search combines the best things about both semantic
/// and lexical search along with statistics from the git log to generate the
/// best code spans which are relevant
pub async fn hybrid_search(
    axumQuery(HybridSearchQuery { query, repo }): axumQuery<HybridSearchQuery>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    // Here we want to do the following:
    // - do a semantic search (normalize it to a score between 0.5 -> 1)
    // - do a lexical search (normalize it to a score between 0.5 -> 1)
    // - get statistics from the git log (normalize it to a score between 0.5 -> 1)
    // hand-waving the numbers here for whatever works for now
    // - final score -> git_log_score * 4 + lexical_search * 2.5 + semantic_search_score
    // - combine the score as following
    let session_id = uuid::Uuid::new_v4();
    let llm_client = Arc::new(LlmClient::codestory_infra());
    let conversation_id = uuid::Uuid::new_v4();
    let sql_db = app.sql.clone();
    let (sender, _) = tokio::sync::mpsc::channel(100);
    let mut agent = Agent::prepare_for_semantic_search(
        app,
        repo,
        session_id,
        &query,
        llm_client,
        conversation_id,
        sql_db,
        sender,
    );
    let hybrid_search_results = agent.code_search_hybrid(&query).await.unwrap_or(vec![]);
    Ok(json(HybridSearchResponse {
        session_id: uuid::Uuid::new_v4(),
        query,
        code_spans: hybrid_search_results,
    }))
}
