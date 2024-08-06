use crate::{
    agentic::{
        symbol::identifier::LLMProperties,
        tool::{
            code_symbol::important::{
                CodeSymbolImportantResponse, CodeSymbolWithSteps, CodeSymbolWithThinking,
            },
            errors::ToolError,
            input::ToolInput,
            kw_search::tag_search::TagSearch,
            output::ToolOutput,
            r#type::Tool,
        },
    },
    repomap::tag::{Tag, TagIndex},
};
use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientError, LLMType},
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use thiserror::Error;

pub struct SearchPlanQuery {
    user_query: String,
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    repo_name: String,
    root_request_id: String,
    context: Vec<SearchPlanContext>,
    // case_sensitive: bool,
    // tag_index: TagIndex,
}

impl SearchPlanQuery {
    pub fn new(
        user_query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        repo_name: String,
        root_request_id: String,
        context: Vec<SearchPlanContext>,
    ) -> Self {
        Self {
            user_query,
            llm,
            provider,
            api_keys,
            repo_name,
            root_request_id,
            context,
        }
    }

    pub fn root_request_id(&self) -> &str {
        &self.root_request_id
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

    pub fn repo_name(&self) -> &str {
        &self.repo_name
    }

    pub fn context(&self) -> &Vec<SearchPlanContext> {
        &self.context
    }
}

#[async_trait]
pub trait GenerateSearchPlan {
    async fn generate_search_plan(
        &self,
        request: &SearchPlanQuery,
    ) -> Result<String, GenerateSearchPlanError>;
}

pub enum SearchPlanContext {
    RepoTree(String),
}

#[derive(Debug, Error)]
pub enum GenerateSearchPlanError {
    #[error("generic error: {0}")]
    Generic(String),
    #[error("LLM Client erorr: {0}")]
    LLMClientError(#[from] LLMClientError),
}
