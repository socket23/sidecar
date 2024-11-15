//! This grabs all the pending output if any from the subprocess which have been spawned

use async_trait::async_trait;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

#[derive(Debug, Clone, serde::Serialize)]
pub struct SubProcessSpawnedPendingOutputRequest {
    busy: bool,
    completed: bool,
    editor_url: String,
}

impl SubProcessSpawnedPendingOutputRequest {
    pub fn with_editor_url(editor_url: String) -> Self {
        Self {
            busy: true,
            completed: true,
            editor_url,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubProcessSpanwedPendingOutputResponse {
    output: Option<String>,
}

impl SubProcessSpanwedPendingOutputResponse {
    pub fn output(self) -> Option<String> {
        self.output
    }
}

pub struct SubProcessSpawnedPendingOutputClient {
    client: reqwest::Client,
}

#[async_trait]
impl Tool for SubProcessSpawnedPendingOutputClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_subprocess_spawn_pending_output()?;
        let editor_endpoint = context.editor_url.to_owned() + "/terminal_output_new";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;

        let response: SubProcessSpanwedPendingOutputResponse = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;

        Ok(ToolOutput::SubProcessSpawnedPendingOutput(response))
    }

    fn tool_description(&self) -> String {
        "".to_owned()
    }

    fn tool_input_format(&self) -> String {
        "".to_owned()
    }
}
