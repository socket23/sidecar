use async_trait::async_trait;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

pub struct TerminalTool {
    client: reqwest::Client,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TerminalInputPartial {
    command: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TerminalInput {
    command: String,
    editor_url: String,
}

impl TerminalInput {
    pub fn new(command: String, editor_url: String) -> Self {
        Self {
            command,
            editor_url,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TerminalOutput {
    output: String,
}

impl TerminalOutput {
    pub fn output(&self) -> &str {
        &self.output
    }
}

impl TerminalTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for TerminalTool {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_terminal_command()?;
        let editor_endpoint = context.editor_url.to_owned() + "/execute_terminal_command";

        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;

        let terminal_response: TerminalOutput = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;

        Ok(ToolOutput::TerminalCommand(terminal_response))
    }

    fn tool_description(&self) -> String {
        "Execute a terminal command".to_owned()
    }

    fn tool_input_format(&self) -> String {
        "".to_owned()
    }
}
