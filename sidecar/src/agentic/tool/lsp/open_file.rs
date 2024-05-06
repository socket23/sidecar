use crate::agentic::tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput};
use async_trait::async_trait;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OpenFileRequest {
    fs_file_path: String,
    editor_url: String,
}

impl OpenFileRequest {
    pub fn new(fs_file_path: String, editor_url: String) -> Self {
        Self {
            fs_file_path,
            editor_url,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OpenFileResponse {
    fs_file_path: String,
    file_contents: String,
}

impl OpenFileResponse {
    pub fn contents(self) -> String {
        self.file_contents
    }
}

pub struct LSPOpenFile {
    client: reqwest::Client,
}

impl LSPOpenFile {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for LSPOpenFile {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        // we want to create a new file open request over here
        let context = input.is_file_open()?;
        // now we send it over to the editor
        let editor_endpoint = context.editor_url.to_owned() + "/file_open";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let response: OpenFileResponse = response
            .json()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        Ok(ToolOutput::FileOpen(response))
    }
}
