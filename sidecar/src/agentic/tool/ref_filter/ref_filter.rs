use std::sync::Arc;

use llm_client::broker::LLMBroker;

use crate::agentic::symbol::identifier::LLMProperties;

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
