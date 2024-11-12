//! Attemps to complete the session and reply to the user with the required output

use async_trait::async_trait;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

pub struct AttemptCompletionClient {}

impl AttemptCompletionClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttemptCompletionClientRequest {
    result: String,
    command: Option<String>,
}

impl AttemptCompletionClientRequest {
    pub fn new(result: String, command: Option<String>) -> Self {
        Self { result, command }
    }
}

#[derive(Debug, Clone)]
pub struct AttemptCompletionClientResponse {
    result: String,
    command: Option<String>,
}

impl AttemptCompletionClientResponse {
    pub fn new(result: String, command: Option<String>) -> Self {
        Self { result, command }
    }

    pub fn reply(&self) -> &str {
        &self.result
    }

    pub fn command(&self) -> Option<String> {
        self.command.clone()
    }
}

#[async_trait]
impl Tool for AttemptCompletionClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_attempt_completion()?;
        let request = context.result.to_owned();
        let command = context.command.clone();
        Ok(ToolOutput::AttemptCompletion(
            AttemptCompletionClientResponse::new(request, command),
        ))
    }

    fn tool_description(&self) -> String {
        r#"After each tool use, the user will respond with the result of that tool use, i.e. if it succeeded or failed, along with any reasons for failure. Once you've received the results of tool uses and can confirm that the task is complete, use this tool to present the result of your work to the user. Optionally you may provide a CLI command to showcase the result of your work. The user may respond with feedback if they are not satisfied with the result, which you can use to make improvements and try again."#.to_owned()
    }

    fn tool_input_format(&self) -> String {
        r#"Parameters:
- result: (required) The result of the task. Formulate this result in a way that is final and does not require further input from the user. Don't end your result with questions or offers for further assistance.
- command: (optional) A CLI command to execute to show a live demo of the result to the user. For example, use \`open index.html\` to display a created html website, or \`open localhost:3000\` to display a locally running development server. But DO NOT use commands like \`echo\` or \`cat\` that merely print text. This command should be valid for the current operating system. Ensure the command is properly formatted and does not contain any harmful instructions.
Usage:
<attempt_completion>
<result>
Your final result description here
</result>
<command>Command to demonstrate result (optional)</command>
</attempt_completion>"#.to_owned()
    }
}
