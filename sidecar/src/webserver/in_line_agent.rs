use std::{sync::Arc, time::Duration};

use axum::{response::sse, Extension, Json};
use rand::seq::SliceRandom;
use regex::Regex;
use serde_json::json;

use super::{in_line_agent_stream::generate_in_line_agent_stream, types::Result};
use crate::{
    agent::llm_funcs::LlmClient,
    application::application::Application,
    chunking::{
        editor_parsing::EditorParsing,
        text_document::{Position, Range},
    },
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

impl SnippetInformation {
    pub fn to_range(&self) -> Range {
        Range::new(self.start_position.clone(), self.end_position.clone())
    }

    pub fn set_start_byte_offset(&mut self, byte_offset: usize) {
        self.start_position.set_byte_offset(byte_offset);
    }

    pub fn set_end_byte_offset(&mut self, byte_offset: usize) {
        self.end_position.set_byte_offset(byte_offset);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentWeb {
    pub text: String,
    pub utf8_array: Vec<u8>,
    pub language: String,
    pub fs_file_path: String,
    pub relative_path: String,
    pub line_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRelatedInformation {
    pub text: String,
    pub language: String,
    pub range: Range,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticInformation {
    pub prompt_parts: Vec<String>,
    pub related_information: Vec<DiagnosticRelatedInformation>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticInformationFromEditor {
    pub first_message: String,
    pub diagnostic_information: Vec<DiagnosticInformation>,
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
    pub diagnostics_information: Option<DiagnosticInformationFromEditor>,
    pub openai_key: Option<String>,
}

impl ProcessInEditorRequest {
    pub fn source_code(&self) -> &str {
        &self.text_document_web.text
    }

    pub fn source_code_bytes(&self) -> &[u8] {
        &self.text_document_web.utf8_array
    }

    pub fn language(&self) -> &str {
        &self.text_document_web.language
    }

    pub fn line_count(&self) -> usize {
        self.text_document_web.line_count
    }

    pub fn start_position(&self) -> Position {
        self.snippet_information.start_position.clone()
    }

    pub fn end_position(&self) -> Position {
        self.snippet_information.end_position.clone()
    }

    pub fn fs_file_path(&self) -> &str {
        &self.text_document_web.fs_file_path
    }
}

pub async fn reply_to_user(
    Extension(app): Extension<Application>,
    Json(ProcessInEditorRequest {
        query,
        language,
        repo_ref,
        mut snippet_information,
        thread_id,
        text_document_web,
        diagnostics_information,
        openai_key,
    }): Json<ProcessInEditorRequest>,
) -> Result<impl IntoResponse> {
    let editor_parsing: EditorParsing = Default::default();
    // Now we want to handle this and send the data to a prompt which will generate
    // the proper things
    // Here we will handle how the in-line agent will handle the work
    let sql_db = app.sql.clone();
    let llm_client = if let Some(user_key_openai) = &openai_key {
        LlmClient::user_key_openai(
            app.posthog_client.clone(),
            app.sql.clone(),
            app.user_id.to_owned(),
            app.llm_config.clone(),
            user_key_openai.to_owned(),
        )
    } else {
        LlmClient::codestory_infra(
            app.posthog_client.clone(),
            app.sql.clone(),
            app.user_id.to_owned(),
            app.llm_config.clone(),
        )
    };
    let (sender, receiver) = tokio::sync::mpsc::channel(100);
    let inline_agent_message = InLineAgentMessage::start_message(thread_id, query.to_owned());
    snippet_information =
        fix_snippet_information(snippet_information, text_document_web.utf8_array.as_slice());
    // we have to fix the snippet information which is coming over the wire
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
            diagnostics_information,
            openai_key,
        },
        vec![inline_agent_message],
        sender,
    );
    let result = generate_in_line_agent_stream(
        inline_agent,
        // Since we are always starting with deciding the action, lets send that
        // as the first action
        in_line_agent::types::InLineAgentAction::DecideAction { query },
        receiver,
    )
    .await?;
    Ok(result.keep_alive(
        sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .event(
                sse::Event::default()
                    .json_data(json!(
                        {"keep_alive": get_keep_alive_message(),
                        "session_id": thread_id,
                    }))
                    .expect("json to not fail on keep alive"),
            ),
    ))
}

fn get_keep_alive_message() -> String {
    [
        "Fetching response... please wait",
        "Aide is hard at work, any moment now...",
        "Code snippets incoming...",
        "Processing code snippets...",
    ]
    .choose(&mut rand::thread_rng())
    .map(|value| value.to_string())
    .unwrap_or("Working on your request...".to_owned())
}

fn line_column_to_byte_offset(
    lines: Vec<&str>,
    target_line: usize,
    target_column: usize,
) -> Option<usize> {
    // Keep track of the current line and column in the input text
    let mut current_line = 0;
    let mut current_byte_offset = 0;

    for (index, line) in lines.iter().enumerate() {
        if index == target_line {
            let mut current_col = 0;

            // If the target column is at the beginning of the line
            if target_column == 0 {
                return Some(current_byte_offset);
            }

            for char in line.chars() {
                if current_col == target_column {
                    return Some(current_byte_offset);
                }
                current_byte_offset += char.len_utf8();
                current_col += 1;
            }

            // If the target column is exactly at the end of this line
            if current_col == target_column {
                return Some(current_byte_offset); // target_column is at the line break
            }

            // Column requested is beyond the current line length
            return None;
        }

        // Increment the byte offset by the length of the current line and its newline
        current_byte_offset += line.len() + "\n".len(); // add 1 for the newline character
        current_line += 1;
    }

    // Line requested is beyond the input text line count
    None
}

pub fn fix_snippet_information(
    mut snippet_information: SnippetInformation,
    text_bytes: &[u8],
) -> SnippetInformation {
    // First we convert from the bytes to the string
    let text_str = String::from_utf8(text_bytes.to_vec()).unwrap_or_default();
    // Now we have to split the text on the new lines
    let re = Regex::new(r"\r\n|\r|\n").unwrap();

    // Split the string using the regex pattern
    let lines: Vec<&str> = re.split(&text_str).collect();

    let start_position_byte_offset = line_column_to_byte_offset(
        lines.to_vec(),
        snippet_information.start_position.line(),
        snippet_information.start_position.column(),
    );

    let end_position_byte_offset = line_column_to_byte_offset(
        lines.to_vec(),
        snippet_information.end_position.line(),
        snippet_information.end_position.column(),
    );

    if let Some(start_position_byte) = start_position_byte_offset {
        snippet_information.set_start_byte_offset(start_position_byte);
    }

    if let Some(end_position_byte) = end_position_byte_offset {
        snippet_information.set_end_byte_offset(end_position_byte);
    }

    snippet_information
}
