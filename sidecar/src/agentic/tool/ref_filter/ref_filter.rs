use async_trait::async_trait;
use llm_client::broker::LLMBroker;
use std::sync::Arc;

use crate::agentic::{
    symbol::identifier::{LLMProperties, SymbolIdentifier},
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReferenceFilterRequest {
    user_instruction: String,
    references: Vec<SymbolIdentifier>, // todo(zi) this needs to be considered.
    llm_properties: LLMProperties,
    root_id: String,
}

impl ReferenceFilterRequest {
    pub fn new(
        user_instruction: String,
        references: Vec<SymbolIdentifier>,
        llm_properties: LLMProperties,
        root_id: String,
    ) -> Self {
        Self {
            user_instruction,
            references,
            llm_properties,
            root_id,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReferenceFilterResponse {
    references: Vec<SymbolIdentifier>,
}

impl ReferenceFilterResponse {
    pub fn new(references: Vec<SymbolIdentifier>) -> Self {
        Self { references }
    }

    pub fn references(&self) -> &[SymbolIdentifier] {
        &self.references
    }
}

pub struct ReferenceFilterBroker {
    llm_client: Arc<LLMBroker>,
    fail_over_llm: LLMProperties,
}

impl ReferenceFilterBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            fail_over_llm,
        }
    }
}

#[async_trait]
impl Tool for ReferenceFilterBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        todo!();
    }
}
