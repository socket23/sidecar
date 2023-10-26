use std::sync::Arc;

use axum::{Extension, Json};

use super::{in_line_agent_stream::generate_in_line_agent_stream, types::Result};
use crate::{
    agent::llm_funcs::LlmClient,
    application::application::Application,
    in_line_agent::{self, types::InLineAgent},
    repo::types::RepoRef,
};
use axum::response::IntoResponse;

/// This module contains all the helper structs which we need to enable in-editor experience

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnippetInformation {
    pub snippet_before: String,
    pub snippet_after: String,
    pub snippet_selected: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessInEditorRequest {
    pub query: String,
    pub language: String,
    pub repo_ref: RepoRef,
    pub snippet_information: SnippetInformation,
    pub thread_id: uuid::Uuid,
}

pub async fn reply_to_user(
    Extension(app): Extension<Application>,
    Json(ProcessInEditorRequest {
        query,
        language,
        repo_ref,
        snippet_information,
        thread_id,
    }): Json<ProcessInEditorRequest>,
) -> Result<impl IntoResponse> {
    // Here we will handle how the in-line agent will handle the work
    let sql_db = app.sql.clone();
    let llm_client = LlmClient::codestory_infra();
    let (sender, receiver) = tokio::sync::mpsc::channel(100);
    let inline_agent = InLineAgent::new(app, repo_ref, sql_db, Arc::new(llm_client), sender);
    generate_in_line_agent_stream(
        inline_agent,
        // Since we are always starting with deciding the action, lets send that
        // as the first action
        in_line_agent::types::InLineAgentAction::DecideAction { query },
        receiver,
    )
    .await
}
