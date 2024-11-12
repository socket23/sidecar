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
    completion_reply: String,
}

impl AttemptCompletionClientRequest {
    pub fn new(completion_reply: String) -> Self {
        Self { completion_reply }
    }
}

#[derive(Debug, Clone)]
pub struct AttemptCompletionClientResponse {
    completion_reply: String,
}

impl AttemptCompletionClientResponse {
    pub fn new(completion_reply: String) -> Self {
        Self { completion_reply }
    }

    pub fn reply(&self) -> &str {
        &self.completion_reply
    }
}

#[async_trait]
impl Tool for AttemptCompletionClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_attempt_completion()?;
        let request = context.completion_reply;
        Ok(ToolOutput::AttemptCompletion(
            AttemptCompletionClientResponse::new(request),
        ))
    }

    fn tool_description(&self) -> String {
        "".to_owned()
    }

    fn tool_input_format(&self) -> String {
        "".to_owned()
    }
}
