use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use std::{collections::HashMap, sync::Arc};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{
        code_symbol::{
            important::CodeSymbolImportantResponse,
            models::anthropic::AnthropicCodeSymbolImportant, types::CodeSymbolError,
        },
        errors::ToolError,
        input::ToolInput,
        output::ToolOutput,
        r#type::Tool,
    },
};

use super::models::google_studio::GoogleStudioBigSearch;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BigSearchRequest {
    llm: LLMType,
    requests: Vec<SearchRequest>,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
}

impl BigSearchRequest {
    pub fn new(
        llm: LLMType,
        requests: Vec<SearchRequest>,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
    ) -> Self {
        Self {
            llm,
            requests,
            provider,
            api_keys,
        }
    }

    pub fn llm(&self) -> &LLMType {
        &self.llm
    }

    pub fn provider(&self) -> &LLMProvider {
        &self.provider
    }

    pub fn api_keys(&self) -> &LLMProviderAPIKeys {
        &self.api_keys
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchRequest {
    root_dir: String,
    user_query: String,
}

#[async_trait]
pub trait BigSearch {
    async fn search(
        &self,
        input: BigSearchRequest,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError>;
}

pub struct BigSearchBroker {
    llms: HashMap<LLMType, Box<dyn BigSearch + Send + Sync>>,
}

impl BigSearchBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        let mut llms: HashMap<LLMType, Box<dyn BigSearch + Send + Sync>> = Default::default();

        llms.insert(
            LLMType::GeminiProFlash,
            Box::new(GoogleStudioBigSearch::new(
                llm_client.clone(),
                fail_over_llm,
            )),
        );

        Self { llms }
    }
}

#[async_trait]
impl Tool for BigSearchBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = input.big_search_query()?;

        if let Some(implementation) = self.llms.get(request.llm()) {
            let output = implementation
                .search(request)
                .await
                .map_err(|e| ToolError::FileImportantError(e.to_string()))?;

            Ok(ToolOutput::BigSearch(output))
        } else {
            Err(ToolError::LLMNotSupported)
        }
    }
}
