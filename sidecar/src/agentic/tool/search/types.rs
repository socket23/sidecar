use async_trait::async_trait;
use llm_client::{broker::LLMBroker, clients::types::LLMType};
use std::{collections::HashMap, sync::Arc};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::code_symbol::{
        important::CodeSymbolImportantResponse, models::anthropic::AnthropicCodeSymbolImportant,
        types::CodeSymbolError,
    },
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BigSearchRequest {
    root_dir: String,
    user_query: String,
}

#[async_trait]
pub trait BigSearch {
    async fn search(
        &self,
        input: Vec<BigSearchRequest>,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError>;
}

pub struct BigSearchBroker {
    llms: HashMap<LLMType, Box<dyn BigSearch + Send + Sync>>,
}

impl BigSearchBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        let mut llms: HashMap<LLMType, Box<dyn BigSearch + Send + Sync>> = Default::default();

        // llms.insert(
        //     LLMType::GeminiProFlash,
        //     Box::new(AnthropicCodeSymbolImportant::new(
        //         llm_client.clone(),
        //         fail_over_llm,
        //     )),
        // );

        Self { llms }
    }
}
