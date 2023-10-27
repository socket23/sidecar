use std::sync::Arc;

use axum::{Extension, Json};

use super::{in_line_agent_stream::generate_in_line_agent_stream, types::Result};
use crate::{
    agent::llm_funcs::LlmClient,
    application::application::Application,
    chunking::{editor_parsing::EditorParsing, text_document::Position},
    in_line_agent::{
        self,
        types::{InLineAgent, InLineAgentMessage},
    },
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
    // Now we want to handle this and send the data to a prompt which will generate
    // the proper things
    // Here we will handle how the in-line agent will handle the work
    let sql_db = app.sql.clone();
    let llm_client = LlmClient::codestory_infra();
    let (sender, receiver) = tokio::sync::mpsc::channel(100);
    let inline_agent_message = InLineAgentMessage::start_message(thread_id, query.to_owned());
    let inline_agent = InLineAgent::new(
        app,
        repo_ref.clone(),
        sql_db,
        Arc::new(llm_client),
        editor_parsing,
        ProcessInEditorRequest {
            query: query.to_owned(),
            language,
            repo_ref,
            snippet_information,
            text_document_web,
            thread_id,
        },
        vec![inline_agent_message],
        sender,
    );
    generate_in_line_agent_stream(
        inline_agent,
        // Since we are always starting with deciding the action, lets send that
        // as the first action
        in_line_agent::types::InLineAgentAction::DecideAction { query },
        receiver,
    )
    .await
}
