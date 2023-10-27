use std::sync::Arc;

use axum::{Extension, Json};

use super::{in_line_agent_stream::generate_in_line_agent_stream, types::Result};
use crate::{
    agent::llm_funcs::LlmClient,
    application::application::Application,
    chunking::{editor_parsing::EditorParsing, text_document::Position},
    in_line_agent::{self, types::InLineAgent},
    repo::types::RepoRef,
};
use axum::response::IntoResponse;

/// This module contains all the helper structs which we need to enable in-editor experience

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetInformation {
    pub start_position: Position,
    pub end_position: Position,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentWeb {
    pub text: String,
    pub language: String,
    pub fs_file_path: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInEditorRequest {
    pub query: String,
    pub language: String,
    pub repo_ref: RepoRef,
    pub snippet_information: SnippetInformation,
    pub text_document_web: TextDocumentWeb,
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
        text_document_web,
    }): Json<ProcessInEditorRequest>,
) -> Result<impl IntoResponse> {
    let editor_parsing: EditorParsing = Default::default();
    let source_str = text_document_web.text;
    let language = &text_document_web.language;
    let relative_path = &text_document_web.relative_path;
    let fs_file_path = &text_document_web.fs_file_path;
    let start_position = snippet_information.start_position;
    let end_position = snippet_information.end_position;
    let document_nodes = editor_parsing.get_documentation_node_for_range(
        &source_str,
        language,
        relative_path,
        fs_file_path,
        &start_position,
        &end_position,
        &repo_ref,
    );
    dbg!(document_nodes);
    unimplemented!("not done yet");
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
