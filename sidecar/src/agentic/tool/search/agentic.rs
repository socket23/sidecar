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
    user_context::types::UserContextError,
};
use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientError, LLMType},
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use serde_xml_rs::from_str;
use std::error::Error;
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
// #[serde(rename = "reply")]
pub struct SearchPlanResponse {
    #[serde(rename = "search_plan")]
    search_plan: String,
    #[serde(rename = "files")]
    files: Vec<String>,
}

impl SearchPlanResponse {
    pub fn parse(response: &str) -> Result<Self, GenerateSearchPlanError> {
        if response.is_empty() {
            return Err(GenerateSearchPlanError::EmptyResponse);
        }

        let reply = response
            .lines()
            .skip_while(|line| !line.contains("<reply>"))
            .skip(1)
            .take_while(|line| !line.contains("</reply>"))
            .collect::<Vec<&str>>()
            .join("\n");

        println!("searchplanresponse::reply: {:?}", reply);

        let parsed_string = from_str::<SearchPlanResponse>(&reply).map_err(|e| {
            GenerateSearchPlanError::SerdeError(SerdeError::new(e, reply.to_string()))
        })?;

        Ok(parsed_string)
    }

    pub fn search_plan(&self) -> &str {
        &self.search_plan
    }

    pub fn files(&self) -> &Vec<String> {
        &self.files
    }
}

#[async_trait]
pub trait GenerateSearchPlan {
    async fn generate_search_plan(
        &self,
        request: &SearchPlanQuery,
    ) -> Result<SearchPlanResponse, GenerateSearchPlanError>;
}

pub enum SearchPlanContext {
    RepoTree(String),
}

#[derive(Debug)]
pub struct SerdeError {
    xml_error: serde_xml_rs::Error,
    content: String,
}

impl std::fmt::Display for SerdeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Serde error: {}\nContent:{}",
            self.xml_error, self.content
        )
    }
}

impl Error for SerdeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.xml_error)
    }
}

impl SerdeError {
    pub fn new(xml_error: serde_xml_rs::Error, content: String) -> Self {
        Self { xml_error, content }
    }
}

#[derive(Debug, Error)]
pub enum GenerateSearchPlanError {
    #[error("LLM Client erorr: {0}")]
    LLMClientError(#[from] LLMClientError),

    #[error("Serde error: {0}")]
    SerdeError(#[from] SerdeError),

    #[error("Quick xml error: {0}")]
    QuickXMLError(#[from] quick_xml::DeError),

    #[error("User context error: {0}")]
    UserContextError(#[from] UserContextError),

    #[error("Exhausted retries")]
    ExhaustedRetries,

    #[error("Empty response")]
    EmptyResponse,

    #[error("Wrong LLM for input: {0}")]
    WrongLLM(LLMType),

    #[error("Wrong format: {0}")]
    WrongFormat(String),
}
