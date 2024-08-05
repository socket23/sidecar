use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{
        code_symbol::{
            important::CodeSymbolImportantResponse,
            models::anthropic::AnthropicCodeSymbolImportant,
            repo_map_search::{RepoMapSearchBroker, RepoMapSearchQuery},
            types::CodeSymbolError,
        },
        errors::ToolError,
        file::{
            file_finder::{ImportantFilesFinderBroker, ImportantFilesFinderQuery},
            models::anthropic::AnthropicFileFinder,
        },
        input::ToolInput,
        output::ToolOutput,
        r#type::Tool,
    },
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchType {
    // Tree(String),
    // Repomap(String),
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
    search_type: SearchType,
}

impl BigSearchRequest {
    pub fn new(
        user_query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        root_directory: Option<String>,
        root_request_id: String,
        search_type: SearchType,
    ) -> Self {
        Self {
            user_query,
            llm,
            provider,
            api_keys,
            root_directory,
            root_request_id,
            search_type,
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
    llm_client: Arc<LLMBroker>,
    fail_over_llm: LLMProperties,
}

impl BigSearchBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            fail_over_llm,
        }
    }

    pub fn llm_client(&self) -> Arc<LLMBroker> {
        self.llm_client.clone()
    }

    pub fn fail_over_llm(&self) -> LLMProperties {
        self.fail_over_llm.clone()
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

        let tree_broker = ImportantFilesFinderBroker::new(self.llm_client(), self.fail_over_llm());
        let tree_input = ToolInput::ImportantFilesFinder(ImportantFilesFinderQuery::new(
            "tree".to_string(),
            request.user_query().to_string(),
            request.llm().clone(),
            request.provider().clone(),
            request.api_keys().clone(),
            "reponame".to_string(),
            request.root_request_id().to_string(),
        ));

        let tree_output = tree_broker.invoke(tree_input).await?; // these are important symbols already...
                                                                 // transpose to codesymbolImportantResponse

        println!("tree_output: {:?}", tree_output);

        let repo_map_broker = RepoMapSearchBroker::new(self.llm_client(), self.fail_over_llm());
        let repo_map_input = ToolInput::RepoMapSearch(RepoMapSearchQuery::new(
            "repo_map".to_string(),
            request.user_query().to_string(),
            request.llm().clone(),
            request.provider().clone(),
            request.api_keys().clone(),
            request.root_directory().map(|d| d.to_string()),
            request.root_request_id().to_string(),
        ));

        let repo_map_output = repo_map_broker.invoke(repo_map_input).await?;

        println!("repo_map_output: {:?}", repo_map_output);

        todo!();
    }
}
