use async_trait::async_trait;
use std::sync::Arc;

use llm_client::broker::LLMBroker;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

// consider possibility of constraining number of steps
#[derive(Debug, Clone)]
pub struct StepGeneratorRequest {
    user_query: String,
    root_request_id: String,
    editor_url: String,
}

impl StepGeneratorRequest {
    pub fn new(user_query: String, root_request_id: String, editor_url: String) -> Self {
        Self {
            user_query,
            root_request_id,
            editor_url,
        }
    }

    pub fn user_query(&self) -> &str {
        &self.user_query
    }

    pub fn root_request_id(&self) -> &str {
        &self.root_request_id
    }

    pub fn editor_url(&self) -> &str {
        &self.editor_url
    }
}

pub struct StepGeneratorClient {
    llm_client: Arc<LLMBroker>,
}

impl StepGeneratorClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }
}

#[async_trait]
impl Tool for StepGeneratorClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        todo!();
    }
}
