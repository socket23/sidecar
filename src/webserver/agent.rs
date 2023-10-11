use axum::response::sse;
use axum::response::Sse;
use futures::future::Either;
use futures::stream;
use futures::FutureExt;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;

use axum::response::IntoResponse;
use axum::{extract::Query, Extension};
/// We will invoke the agent to get the answer, we are moving to an agent based work
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::llm_funcs::LlmClient;
use crate::agent::types::Agent;
use crate::agent::types::AgentAction;
use crate::agent::types::ConversationMessage;
use crate::application::application::Application;
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
    Query(SearchInformation { query, reporef }): Query<SearchInformation>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    let session_id = uuid::Uuid::new_v4();
    let llm_client = Arc::new(LlmClient::codestory_infra());
    let conversation_id = uuid::Uuid::new_v4();
    let sql_db = app.sql.clone();

    // Process the events in parallel here
    let conversation_message_stream = async_stream::try_stream! {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
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

            let left_stream = (&mut conversation_message_stream).map(Either::Left);
            let right_stream = agent
                .iterate(action)
                .into_stream()
                .map(Either::Right);

            let timeout = Duration::from_secs(TIMEOUT_SECS);

            let mut next = None;
            for await item in tokio_stream::StreamExt::timeout(
                stream::select(left_stream, right_stream),
                timeout,
            ) {
                match item {
                    Ok(Either::Left(conversation_message)) => yield conversation_message,
                    Ok(Either::Right(next_action)) => match next_action {
                        Ok(n) => break next = n,
                        Err(e) => break 'outer Err(anyhow::anyhow!(e)),
                    },
                    Err(_) => break 'outer Err(anyhow::anyhow!("timeout")),
                }
            }

            // If we have some elements which are still present in the stream, we
            // return them here so as to not loose things in case the timeout got triggered
            // this is basically draining the stream properly
            while let Some(Some(conversation_message)) = conversation_message_stream.next().now_or_never() {
                yield conversation_message;
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
