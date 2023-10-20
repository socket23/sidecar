use super::agent_stream::generate_agent_stream;
use super::types::json;
use anyhow::Context;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract::Query as axumQuery, Extension};
/// We will invoke the agent to get the answer, we are moving to an agent based work
use serde::{Deserialize, Serialize};

use crate::agent::llm_funcs::LlmClient;
use crate::agent::model::GPT_4;
use crate::agent::types::Agent;
use crate::agent::types::AgentAction;
use crate::agent::types::CodeSpan;
use crate::agent::types::ConversationMessage;
use crate::application::application::Application;
use crate::indexes::code_snippet::CodeSnippetDocument;
use crate::repo::types::RepoRef;

use super::types::ApiResponse;
use super::types::Result;

fn default_thread_id() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SearchInformation {
    pub query: String,
    pub reporef: RepoRef,
    #[serde(default = "default_thread_id")]
    pub thread_id: uuid::Uuid,
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
    axumQuery(SearchInformation {
        query,
        reporef,
        thread_id,
    }): axumQuery<SearchInformation>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    let session_id = uuid::Uuid::new_v4();
    let llm_client = Arc::new(LlmClient::codestory_infra());
    let sql_db = app.sql.clone();
    let (sender, receiver) = tokio::sync::mpsc::channel(100);
    let action = AgentAction::Query(query.clone());
    let previous_conversation_message =
        ConversationMessage::load_from_db(sql_db.clone(), &reporef, thread_id)
            .await
            .expect("loading from db to never fail");
    let agent = Agent::prepare_for_search(
        app,
        reporef,
        session_id,
        &query,
        llm_client,
        thread_id,
        sql_db,
        previous_conversation_message,
        sender,
    );

    generate_agent_stream(agent, action, receiver).await
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
        vec![], // we don't have a previous conversation message here
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
        vec![], // we don't have a previous conversation message here
        sender,
    );
    let hybrid_search_results = agent.code_search_hybrid(&query).await.unwrap_or(vec![]);
    Ok(json(HybridSearchResponse {
        session_id: uuid::Uuid::new_v4(),
        query,
        code_spans: hybrid_search_results,
    }))
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ExplainRequest {
    query: String,
    relative_path: String,
    start_line: u64,
    end_line: u64,
    repo_ref: RepoRef,
    #[serde(default = "default_thread_id")]
    thread_id: uuid::Uuid,
}

/// We are going to handle the explain function here, but its going to be very
/// bare-bones right now. We don't give the user the option to explore or do
/// more things with the agent yet, ideal explain feature will be when the user
/// gets to explore the repository or maybe that can be a different UX like the
/// crawler
pub async fn explain(
    axumQuery(ExplainRequest {
        query,
        relative_path,
        start_line,
        end_line,
        repo_ref,
        thread_id,
    }): axumQuery<ExplainRequest>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    let file_content = app
        .indexes
        .file
        .get_by_path(&relative_path, &repo_ref)
        .await
        .context("file retrieval failed")?
        .context("requested file not found")?
        .content;

    let mut previous_messages =
        ConversationMessage::load_from_db(app.sql.clone(), &repo_ref, thread_id)
            .await
            .expect("loading from db to never fail");

    let snippet = file_content
        .lines()
        .skip(start_line.try_into().expect("conversion_should_not_fail"))
        .take(
            (end_line - start_line)
                .try_into()
                .expect("conversion_should_not_fail"),
        )
        .collect::<Vec<_>>()
        .join("\n");

    let mut conversation_message = ConversationMessage::explain_message(
        thread_id,
        crate::agent::types::AgentState::Explain,
        query,
    );

    let code_span = CodeSpan {
        file_path: relative_path.to_owned(),
        alias: 0,
        start_line,
        end_line,
        data: snippet,
        score: Some(1.0),
    };
    conversation_message.add_user_selected_code_span(code_span.clone());
    conversation_message.add_code_spans(code_span.clone());
    conversation_message.add_path(relative_path);

    previous_messages.push(conversation_message);

    let action = AgentAction::Answer { paths: vec![0] };

    let (sender, receiver) = tokio::sync::mpsc::channel(100);

    let session_id = uuid::Uuid::new_v4();

    let sql = app.sql.clone();

    let agent = Agent {
        application: app,
        reporef: repo_ref,
        session_id,
        conversation_messages: previous_messages,
        llm_client: Arc::new(LlmClient::codestory_infra()),
        model: GPT_4,
        sql_db: sql,
        sender,
    };

    generate_agent_stream(agent, action, receiver).await
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FollowupChatRequest {
    pub query: String,
    pub repo_ref: RepoRef,
    pub thread_id: uuid::Uuid,
}

pub async fn followup_chat(
    axumQuery(FollowupChatRequest {
        query,
        repo_ref,
        thread_id,
    }): axumQuery<FollowupChatRequest>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    let session_id = uuid::Uuid::new_v4();
    // Here we do something special, if the user is asking a followup question
    // we just look at the previous conversation message the thread belonged
    // to and use that as context for grounding the agent response. In the future
    // we can obviously add more context using @ symbols etc
    let sql_db = app.sql.clone();
    let mut previous_messages =
        ConversationMessage::load_from_db(sql_db.clone(), &repo_ref, thread_id)
            .await
            .expect("loading from db to never fail");

    let mut conversation_message = ConversationMessage::general_question(
        thread_id,
        crate::agent::types::AgentState::FollowupChat,
        query.to_owned(),
    );

    // Now we check if we have any previous messages, if we do we have to signal
    // that to the agent that this could be a followup question, and if we don't
    // know about that then its totally fine
    if let Some(previous_message) = previous_messages.last() {
        previous_message.get_paths().iter().for_each(|path| {
            conversation_message.add_path(path.to_owned());
        });
        previous_message.code_spans().iter().for_each(|code_span| {
            conversation_message.add_code_spans(code_span.clone());
        });
        previous_message
            .user_selected_code_spans()
            .iter()
            .for_each(|code_span| {
                conversation_message.add_user_selected_code_span(code_span.clone())
            });
    }

    previous_messages.push(conversation_message);

    dbg!(&previous_messages);

    let (sender, receiver) = tokio::sync::mpsc::channel(100);

    // If this is a followup, right now we don't take in any additional context,
    // but only use the one from our previous conversation
    let action = AgentAction::Answer { paths: vec![] };
    let agent = Agent::prepare_for_followup(
        app,
        repo_ref,
        session_id,
        Arc::new(LlmClient::codestory_infra()),
        sql_db,
        previous_messages,
        sender,
    );

    generate_agent_stream(agent, action, receiver).await
}
