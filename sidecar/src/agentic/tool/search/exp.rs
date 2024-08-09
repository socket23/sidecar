use llm_client::{
    clients::types::{LLMClientError, LLMType},
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use walkdir::WalkDir;

use std::path::{Path, PathBuf};
use thiserror::Error;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    agentic::tool::code_symbol::important::CodeSymbolImportantResponse,
    repomap::tag::{SearchMode, TagIndex},
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
    pub tool: SearchToolType,
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
    pub fn new(tool: SearchToolType, query: String, thinking: String) -> Self {
        Self {
            tool,
            query,
            thinking,
        }
    }
}

// todo(zi): think about this structure
#[derive(Debug, Clone)]
struct SearchResult {
    path: PathBuf,
    thinking: String,
    snippet: String, // potentially useful for stronger reasoning
}

impl SearchResult {
    pub fn new(path: PathBuf, thinking: &str, snippet: &str) -> Self {
        Self {
            path,
            thinking: thinking.to_string(),
            snippet: snippet.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Repository {
    tree: String,
    outline: String,
    tag_index: TagIndex,
    root: PathBuf,
}

impl Repository {
    pub fn new(tree: String, outline: String, tag_index: TagIndex, root: PathBuf) -> Self {
        Self {
            tree,
            outline,
            tag_index,
            root,
        }
    }

    // todo(zi): file index would be useful here. Considered using tag_index's file_to_tags,
    // but this would mean we'd always ignore .md files, which could contain useful information
    fn find_file(&self, target: &str) -> Option<String> {
        WalkDir::new(&self.root)
            .into_iter()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy() == target)
            .map(|e| e.path().to_string_lossy().into_owned())
    }

    fn execute_search(&self, search_query: &SearchQuery) -> Vec<SearchResult> {
        // Implement repository search logic
        println!("repository::execute_search::query: {:?}", search_query);

        match search_query.tool {
            SearchToolType::File => {
                println!("repository::execute_search::query::SearchToolType::File");

                let file = self.find_file(&search_query.query);

                println!(
                    "repository::execute_search::query::SearchToolType::File::file: {:?}",
                    file
                );

                vec![SearchResult::new(
                    PathBuf::from(file.unwrap_or("".to_string())),
                    &search_query.thinking,
                    "",
                )]
            } // maybe give the thinking to TreeSearch...?
            SearchToolType::Keyword => {
                println!("repository::execute_search::query::SearchToolType::Keyword");

                let result = self.tag_index.search_definitions_flattened(
                    &search_query.query,
                    false,
                    SearchMode::ExactTagName,
                );

                result
                    .iter()
                    .map(|r| SearchResult::new(r.fname.to_owned(), &search_query.thinking, &r.name))
                    .collect()
            }
        }
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

            // todo(zi): this could be async
            let search_results: Vec<SearchResult> = search_queries
                .iter()
                // maybe flat_mapping here works better
                .flat_map(|q| self.repository.execute_search(q))
                .collect();

            println!(
                "{}",
                search_results
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<String>>()
                    .join("\n")
            );

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
