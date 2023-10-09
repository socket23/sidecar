use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract::Query, Extension};
/// We will invoke the agent to get the answer, we are moving to an agent based work
use serde::{Deserialize, Serialize};

use crate::agent::llm_funcs::LlmClient;
use crate::agent::types::{Agent, ConversationMessage};
use crate::application::application::Application;
use crate::repo::types::RepoRef;

use super::types::json;
use super::types::ApiResponse;
use super::types::Result;

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

pub async fn search_agent(
    Query(SearchInformation { query, reporef }): Query<SearchInformation>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    let session_id = uuid::Uuid::new_v4();
    let llm_client = Arc::new(LlmClient::codestory_infra());
    let conversation_id = uuid::Uuid::new_v4();
    let mut agent = Agent::prepare_for_search(
        app,
        reporef,
        session_id,
        &query,
        llm_client,
        conversation_id,
    );
    let _ = agent
        .iterate(crate::agent::types::AgentAction::Query(query.to_owned()))
        .await;
    let last_message = agent.get_last_conversation_message();
    let answer = last_message.answer().to_owned();
    Ok(json(SearchResponse {
        query: query.to_owned(),
        answer: answer.unwrap_or("not_found".to_owned()),
    }))
}
