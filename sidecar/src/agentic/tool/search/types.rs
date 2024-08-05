use async_trait::async_trait;
use llm_client::{
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::agentic::tool::{
    code_symbol::{important::CodeSymbolImportantResponse, types::CodeSymbolError},
    errors::ToolError,
    input::ToolInput,
    output::ToolOutput,
    r#type::Tool,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchType {
    Tree(String),
    Repomap(String),
    Both(String, String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BigSearchRequest {
    user_query: String,
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    root_directory: Option<String>,
    root_request_id: String,
}

impl BigSearchRequest {
    pub fn new(
        user_query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        root_directory: Option<String>,
        root_request_id: String,
    ) -> Self {
        Self {
            user_query,
            llm,
            provider,
            api_keys,
            root_directory,
            root_request_id,
        }
    }

    pub fn user_query(&self) -> &str {
        &self.user_query
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

    pub fn root_directory(&self) -> Option<&str> {
        self.root_directory.as_deref()
    }

    pub fn root_request_id(&self) -> &str {
        &self.root_request_id
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
    strategies: Vec<Box<dyn BigSearch + Send + Sync>>,
}

impl BigSearchBroker {
    pub fn new() -> Self {
        Self { strategies: vec![] }
    }

    pub fn with_strategy(self, strategy: impl BigSearch + Send + Sync + 'static) -> Self {
        Self {
            strategies: {
                let mut strategies = self.strategies;
                strategies.push(Box::new(strategy));
                strategies
            },
        }
    }

    pub async fn search(
        &self,
        input: BigSearchRequest,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError> {
        let mut output: Vec<CodeSymbolImportantResponse> = vec![];
        for strategy in &self.strategies {
            let result = strategy.search(input.clone()).await;
            if let Ok(result) = result {
                println!("BigSearchBroker::search::strategy::search: {:?}", result);
                output.push(result);
            }
        }

        let output = CodeSymbolImportantResponse::merge(output);

        println!("BigSearchBroker::search::output: {:?}", output);

        Err(CodeSymbolError::NoStrategyFound)
    }
}

#[async_trait]
impl Tool for BigSearchBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = match input {
            ToolInput::BigSearch(req) => req,
            _ => {
                return Err(ToolError::BigSearchError(
                    "Expected BigSearch input".to_string(),
                ))
            }
        };

        let result = self
            .search(request)
            .await
            .map_err(|e| ToolError::CodeSymbolError(e))?;

        Ok(ToolOutput::BigSearch(result))
    }
}
