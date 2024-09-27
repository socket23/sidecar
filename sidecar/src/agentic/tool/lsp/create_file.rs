//! Creates the file using the editor endpoint

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFileRequest {
    fs_file_path: String,
    editor_url: String,
}

impl CreateFileRequest {
    pub fn new(fs_file_path: String, editor_url: String) -> Self {
        Self {
            fs_file_path,
            editor_url,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFileResponse {
    done: bool,
    fs_file_path: String,
}

impl CreateFileResponse {
    pub fn new(done: bool, fs_file_path: String) -> Self {
        Self { done, fs_file_path }
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }
}

pub struct LSPCreateFile {
    client: reqwest::Client,
}

impl LSPCreateFile {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for LSPCreateFile {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_file_create()?;
        let editor_endpoint = context.editor_url.to_owned() + "/create_file";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let response: CreateFileResponse = response
            .json()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        Ok(ToolOutput::FileCreate(response))
    }
}
