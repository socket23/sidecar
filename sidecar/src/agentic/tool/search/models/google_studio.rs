use std::sync::Arc;

use llm_client::broker::LLMBroker;

use async_trait::async_trait;

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{
        code_symbol::{important::CodeSymbolImportantResponse, types::CodeSymbolError},
        search::types::{BigSearch, BigSearchRequest},
    },
};

pub struct GoogleBigSearch {
    llm_client: Arc<LLMBroker>,
    fail_over_llm: LLMProperties,
}

impl GoogleBigSearch {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            fail_over_llm,
        }
    }
}

#[async_trait]
impl BigSearch for GoogleBigSearch {
    async fn search(
        &self,
        input: Vec<BigSearchRequest>,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError> {
        todo!();
    }
}
