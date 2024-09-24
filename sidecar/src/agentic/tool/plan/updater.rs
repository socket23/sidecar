use async_trait::async_trait;
use llm_client::broker::LLMBroker;
use std::sync::Arc;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

pub struct PlanUpdateRequest {}

pub struct PlanUpdaterClient {
    llm_client: Arc<LLMBroker>,
}

impl PlanUpdaterClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }
}

#[async_trait]
impl Tool for PlanUpdaterClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        todo!()
    }
}
