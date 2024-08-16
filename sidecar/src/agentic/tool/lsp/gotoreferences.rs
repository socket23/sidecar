use async_trait::async_trait;

use crate::{
    agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
    chunking::text_document::{Position, Range},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoToReferencesRequest {
    fs_file_path: String,
    position: Position,
    editor_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReferenceLocation {
    fs_file_path: String,
    range: Range,
}

impl ReferenceLocation {
    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn range(&self) -> &Range {
        &self.range
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoToReferencesResponse {
    reference_locations: Vec<ReferenceLocation>,
}

impl GoToReferencesResponse {
    pub fn locations(self) -> Vec<ReferenceLocation> {
        self.reference_locations
    }
}

impl GoToReferencesRequest {
    pub fn new(fs_file_path: String, position: Position, editor_url: String) -> Self {
        Self {
            fs_file_path,
            position,
            editor_url,
        }
    }
}

pub struct LSPGoToReferences {
    client: reqwest::Client,
}

impl LSPGoToReferences {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for LSPGoToReferences {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.reference_request()?;
        let editor_endpoint = context.editor_url.to_owned() + "/go_to_references";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let response: GoToReferencesResponse = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;
        Ok(ToolOutput::go_to_reference(response))
    }
}
