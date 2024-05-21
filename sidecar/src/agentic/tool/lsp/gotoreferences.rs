use async_trait::async_trait;

use crate::{
    agentic::tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput},
    chunking::text_document::{Position, Range},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoToReferencesRequest {
    fs_file_path: String,
    position: Position,
    editor_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefereneceLocation {
    fs_file_path: String,
    range: Range,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoToReferencesResponse {
    references_location: Vec<RefereneceLocation>,
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
        let editor_endpoint = context.editor_url.to_owned() + "/go_to_reference";
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
