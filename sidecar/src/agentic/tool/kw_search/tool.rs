use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use thiserror::Error;

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

use super::{
    google_studio::GoogleStudioKeywordSearch,
    types::{KeywordsReply, KeywordsReplyError},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeywordSearchQuery {
    user_query: String,
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    repo_name: String,
    root_request_id: String,
    case_sensitive: bool,
}

impl KeywordSearchQuery {
    pub fn new(
        user_query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        repo_name: String,
        root_request_id: String,
        case_sensitive: bool,
    ) -> Self {
        Self {
            user_query,
            llm,
            provider,
            api_keys,
            repo_name,
            root_request_id,
            case_sensitive,
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

    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }
}

pub struct KeywordSearchQueryResponse {
    words: Vec<String>,
}

#[derive(Debug, Error)]
pub enum KeywordSearchQueryError {
    #[error("Wrong LLM for input: {0}")]
    WrongLLM(LLMType),
}

#[async_trait]
pub trait KeywordSearch {
    async fn get_keywords(
        &self,
        request: KeywordSearchQuery,
    ) -> Result<KeywordsReply, KeywordsReplyError>;
}

pub struct KeywordSearchQueryBroker {
    llms: HashMap<LLMType, Box<dyn KeywordSearch + Send + Sync>>,
}

impl KeywordSearchQueryBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        let mut llms: HashMap<LLMType, Box<dyn KeywordSearch + Send + Sync>> = Default::default();

        // flash all the wayyyy
        llms.insert(
            LLMType::GeminiProFlash,
            Box::new(GoogleStudioKeywordSearch::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );

        Self { llms }
    }
}

#[async_trait]
impl Tool for KeywordSearchQueryBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = input.keyword_search_query()?;
        todo!();
    }
}
