//! The thinking tool for the agent, where it gets to explore a bit about the problem
//! space and come up with plans

use std::sync::Arc;

use async_trait::async_trait;
use llm_client::broker::LLMBroker;

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

pub struct BeforeCodeEditThinkingRequest {
    llm_properties: LLMProperties,
    original_user_query: String,
    plan: String,
    symbol_content: String,
    content_prefix: String,
    context_suffix: String,
}

// This probably needs to run in a loop kind of, cause we want to either exhaust
// the search space or stop at some point, if we keep varying this to an extent
// we should be able to get all the information
// we really need to start keeping history somewhere
pub struct BeforeCodeEditThinkingResponse {
    // we will probably get symbols which we have to ask questions to
    questions_to_ask: Vec<()>,
    steps_to_take_after: Vec<()>,
}

pub struct Thinking {
    llm_broker: Arc<LLMBroker>,
}

#[async_trait]
impl Tool for Thinking {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        todo!("")
    }
}
