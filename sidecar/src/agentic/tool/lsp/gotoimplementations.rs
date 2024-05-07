use crate::{
    agentic::tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput},
    chunking::text_document::{Position, Range},
};
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoToImplementationRequest {
    fs_file_path: String,
    position: Position,
    editor_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImplementationLocation {
    fs_file_path: String,
    range: Range,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoToImplementationResponse {
    implementation_locations: Vec<ImplementationLocation>,
}

pub struct LSPGoToImplementation {
    client: reqwest::Client,
}

impl LSPGoToImplementation {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for LSPGoToImplementation {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.symbol_implementations()?;
        let editor_endpoint = context.editor_url.to_owned() + "/go_to_implementation";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let response: GoToImplementationResponse = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;
        Ok(ToolOutput::go_to_implementation(response))
    }
}
