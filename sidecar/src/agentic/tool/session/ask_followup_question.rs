//! Asks followup questions to the user

use async_trait::async_trait;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

pub struct AskFollowupQuestions {}

impl AskFollowupQuestions {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AskFollowupQuestionsRequest {
    user_question: String,
}

impl AskFollowupQuestionsRequest {
    pub fn new(user_question: String) -> Self {
        Self { user_question }
    }
}

#[derive(Debug, Clone)]
pub struct AskFollowupQuestionsResponse {
    user_question: String,
}

impl AskFollowupQuestionsResponse {
    pub fn user_question(&self) -> &str {
        &self.user_question
    }
}

impl AskFollowupQuestionsResponse {
    pub fn new(user_question: String) -> Self {
        Self { user_question }
    }
}

#[async_trait]
impl Tool for AskFollowupQuestions {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_ask_followup_questions()?;
        let response = AskFollowupQuestionsResponse::new(context.user_question);
        Ok(ToolOutput::AskFollowupQuestions(response))
    }

    fn tool_description(&self) -> String {
        "".to_owned()
    }

    fn tool_input_format(&self) -> String {
        "".to_owned()
    }
}
