use llm_client::{
    clients::types::{LLMClientError, LLMType},
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use std::path::PathBuf;
use thiserror::Error;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    agentic::tool::code_symbol::important::CodeSymbolImportantResponse, repomap::tag::TagIndex,
    user_context::types::UserContextError,
};

use super::agentic::SerdeError;

#[derive(Debug, Clone)]
pub struct Context {
    files: Vec<File>,
    user_query: String,
    thoughts: String,
}

impl Context {
    pub fn new(files: Vec<File>, user_query: String, thoughts: String) -> Self {
        Self {
            files,
            user_query,
            thoughts,
        }
    }

    pub fn files(&self) -> &[File] {
        &self.files
    }

    pub fn file_paths_as_strings(&self) -> Vec<String> {
        self.files
            .iter()
            .map(|f| f.path().to_string_lossy().into_owned())
            .collect()
    }

    pub fn user_query(&self) -> &str {
        &self.user_query
    }

    pub fn thoughts(&self) -> &str {
        &self.thoughts
    }
}

#[derive(Debug, Clone)]
pub struct File {
    path: PathBuf,
    // content: String,
    // preview: String,
}

impl File {
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SearchToolType {
    File,
    Keyword,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub thinking: String,
    pub search_tool: SearchToolType,
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "search_requests")]
pub struct SearchRequests {
    #[serde(rename = "request")]
    pub requests: Vec<SearchQuery>,
}

#[derive(Debug, Error)]
pub enum IterativeSearchError {
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

impl SearchQuery {
    pub fn new(search_tool: SearchToolType, query: String, thinking: String) -> Self {
        Self {
            search_tool,
            query,
            thinking,
        }
    }
}

// todo(zi): think about this structure
struct SearchResult {
    files: Vec<File>,
}

#[derive(Debug, Clone)]
pub struct Repository {
    tree: String,
    outline: String,
    tag_index: TagIndex,
}

impl Repository {
    pub fn new(tree: String, outline: String, tag_index: TagIndex) -> Self {
        Self {
            tree,
            outline,
            tag_index,
        }
    }

    fn execute_search(&self, query: &SearchQuery) -> SearchResult {
        // Implement repository search logic
        println!("repository::execute_search::query: {:?}", query);
        todo!();
        SearchResult { files: Vec::new() }
    }
}

pub struct IterativeSearchQuery {
    context: Context,
    repository: Repository,
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    root_directory: String,
    root_request_id: String,
}

impl IterativeSearchQuery {
    pub fn new(
        context: Context,
        repository: Repository,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        root_directory: String,
        root_request_id: String,
    ) -> Self {
        Self {
            context,
            repository,
            llm,
            provider,
            api_keys,
            root_directory,
            root_request_id,
        }
    }
}

#[async_trait]
pub trait LLMOperations {
    async fn generate_search_query(
        &self,
        context: &Context,
    ) -> Result<Vec<SearchQuery>, IterativeSearchError>;
    // fn identify_relevant_results(
    //     &self,
    //     context: &Context,
    //     search_result: &SearchResult,
    // ) -> Vec<RelevantFile>;
    // fn decide_continue_search(&self, context: &Context) -> bool;
}

// Main system struct
pub struct IterativeSearchSystem<T: LLMOperations> {
    context: Context,
    repository: Repository,
    llm_ops: T,
}

impl<T: LLMOperations> IterativeSearchSystem<T> {
    pub fn new(context: Context, repository: Repository, llm_ops: T) -> Self {
        Self {
            context,
            repository,
            llm_ops,
        }
    }

    fn context(&self) -> &Context {
        &self.context
    }

    pub async fn run(&mut self) -> Result<CodeSymbolImportantResponse, IterativeSearchError> {
        let mut count = 0;
        while count < 1 {
            println!("run loop #{}", count);
            let search_queries = self.search().await?;

            let search_results: Vec<SearchResult> = search_queries
                .iter()
                .map(|q| self.repository.execute_search(q))
                .collect();

            todo!();
            // self.identify(&search_result);
            if !self.decide() {
                break;
            }

            count += 1;
        }

        todo!();
    }

    // this generates search queries
    async fn search(&self) -> Result<Vec<SearchQuery>, IterativeSearchError> {
        self.llm_ops.generate_search_query(self.context()).await
    }

    fn identify(&mut self, search_result: &SearchResult) {
        // Implement identify logic
        // Filter relevant results and add to self.context.files
    }

    fn decide(&mut self) -> bool {
        // Implement decision logic
        // Update self.context.thoughts
        // Return true if more searching is needed, false otherwise
        true
    }
}
