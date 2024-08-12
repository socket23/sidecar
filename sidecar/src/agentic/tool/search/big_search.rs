use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::{
    agentic::{
        symbol::identifier::LLMProperties,
        tool::{
            code_symbol::{important::CodeSymbolImportantResponse, types::CodeSymbolError},
            errors::ToolError,
            input::ToolInput,
            output::ToolOutput,
            r#type::Tool,
            search::{
                google_studio::GoogleStudioLLM,
                iterative::{IterativeSearchContext, IterativeSearchSystem},
                repository::Repository,
            },
        },
    },
    repomap::tag::TagIndex,
    tree_printer::tree::TreePrinter,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchType {
    Both,
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
        let start = Instant::now();
        let request = input.big_search_query()?;

        let root_directory = match request.root_directory() {
            Some(dir) => dir,
            None => {
                return Err(ToolError::BigSearchError(
                    "Root directory is required".to_string(),
                ))
            }
        };

        let tree =
            TreePrinter::to_string_stacked(Path::new(root_directory)).unwrap_or("".to_owned());

        let tag_index = TagIndex::from_path(Path::new(root_directory)).await;

        let repository = Repository::new(
            tree,
            "outline".to_owned(),
            tag_index,
            PathBuf::from(request.root_directory().unwrap_or("").to_string()),
        );

        let iterative_search_context =
            IterativeSearchContext::new(Vec::new(), request.user_query().to_owned(), "".to_owned());

        // google llm operations
        let google_studio_llm_config = GoogleStudioLLM::new(
            request.root_directory().unwrap_or("").to_owned(),
            self.llm_client(),
            request.root_request_id().to_owned(),
        );

        let mut system = IterativeSearchSystem::new(
            iterative_search_context,
            repository,
            google_studio_llm_config,
        );

        let results = system
            .run()
            .await
            .map_err(|e| ToolError::IterativeSearchError(e))?;

        let duration = start.elapsed();
        println!("BigSearchBroker::invoke::duration: {:?}", duration);

        Ok(ToolOutput::BigSearch(results))
    }
}
