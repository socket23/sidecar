use std::path::PathBuf;

use gix::discover::repository;
use llm_client::{
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::repomap::tag::TagIndex;

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

    fn execute_search(&self, query: SearchQuery) -> SearchResult {
        // Implement repository search logic
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
    async fn generate_search_query(&self, context: &Context) -> SearchQuery;
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

    pub async fn run(&mut self) {
        let mut count = 0;
        while count < 1 {
            println!("run loop #{}", count);
            let search_query = self.search().await;
            let search_result = self.repository.execute_search(search_query);
            self.identify(&search_result);
            if !self.decide() {
                break;
            }

            count += 1;
        }
    }

    // this generates the search_query based on context
    async fn search(&self) -> SearchQuery {
        let _ = self.llm_ops.generate_search_query(self.context()).await;

        // execute_search (on repo)

        todo!();
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
