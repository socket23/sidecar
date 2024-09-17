//! The edited files and the git-diff which is ordered by timestamp
//! The idea is that the file which we are editing can go last

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize)]
pub struct EditedFilesRequest {
    editor_url: String,
}

impl EditedFilesRequest {
    pub fn new(editor_url: String) -> Self {
        Self { editor_url }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EditedGitDiffFile {
    fs_file_path: String,
    diff: String,
    updated_timestamp_ms: i64,
}

impl EditedGitDiffFile {
    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn diff(&self) -> &str {
        &self.diff
    }

    pub fn updated_tiemstamp_ms(&self) -> i64 {
        self.updated_timestamp_ms
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EditedFilesResponse {
    changed_files: Vec<EditedGitDiffFile>,
}

impl EditedFilesResponse {
    pub fn changed_files(self) -> Vec<EditedGitDiffFile> {
        self.changed_files
    }
}

pub struct EditedFiles {
    client: reqwest::Client,
}

impl EditedFiles {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for EditedFiles {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.should_edited_files()?;
        let editor_endpoint = context.editor_url.to_owned() + "/recent_edits";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let response: EditedFilesResponse = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;
        Ok(ToolOutput::edited_files(response))
    }
}
