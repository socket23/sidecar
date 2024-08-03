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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchType {
    Tree(String),
    Repomap(String),
    Both(String, String),
}

use super::models::google_studio::GoogleStudioBigSearch;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BigSearchRequest {
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    root_request_id: String,
    repo_name: String,
    user_query: String,
    root_dir: String,
    search_type: SearchType,
}

impl BigSearchRequest {
    pub fn new(
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        root_request_id: String,
        repo_name: String,
        user_query: String,
        root_dir: String,
        search_type: SearchType,
    ) -> Self {
        Self {
            llm,
            provider,
            api_keys,
            root_request_id,
            repo_name,
            user_query,
            root_dir,
            search_type,
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

    pub fn root_request_id(&self) -> &String {
        &self.root_request_id
    }

    pub fn repo_name(&self) -> &String {
        &self.repo_name
    }

    pub fn user_query(&self) -> &String {
        &self.user_query
    }

    pub fn root_dir(&self) -> &String {
        &self.root_dir
    }

    pub fn search_type(&self) -> &SearchType {
        &self.search_type
    }
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
                .map_err(|e| ToolError::CodeSymbolError(e))?;

            Ok(ToolOutput::BigSearch(output))
        } else {
            Err(ToolError::LLMNotSupported)
        }
    }
}
