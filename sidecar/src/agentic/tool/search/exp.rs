use llm_client::{
    clients::types::{LLMClientError, LLMType},
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use serde_xml_rs::to_string;
use walkdir::WalkDir;

use std::path::{Path, PathBuf};
use thiserror::Error;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    agentic::tool::code_symbol::important::{
        CodeSymbolImportantResponse, CodeSymbolWithSteps, CodeSymbolWithThinking,
    },
    repomap::tag::{SearchMode, TagIndex},
    user_context::types::UserContextError,
};

use super::{
    agentic::SerdeError,
    decide::DecideResponse,
    google_studio::GoogleStudioLLM,
    identify::{IdentifiedFile, IdentifyResponse},
    repository::Repository,
};

#[derive(Debug, Clone)]
pub struct Context {
    files: Vec<File>,
    user_query: String,
    scatch_pad: String,
}

impl Context {
    pub fn new(files: Vec<File>, user_query: String, scatch_pad: String) -> Self {
        Self {
            files,
            user_query,
            scatch_pad,
        }
    }

    pub fn files(&self) -> &[File] {
        &self.files
    }

    pub fn add_files(&mut self, files: Vec<File>) {
        self.files.extend(files)
    }

    pub fn file_paths_as_strings(&self) -> Vec<String> {
        self.files
            .iter()
            .map(|f| f.path().to_string_lossy().into_owned())
            .collect()
    }

    // todo(zi): consider extending thoughts over replacing
    pub fn update_scatch_pad(&mut self, scatch_pad: &str) {
        self.scatch_pad = scatch_pad.to_string()
    }

    pub fn user_query(&self) -> &str {
        &self.user_query
    }

    pub fn scatch_pad(&self) -> &str {
        &self.scatch_pad
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    path: PathBuf,
    // content: String,
    // preview: String,
}

impl File {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            path: path.to_owned(),
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn serialise_files(files: &[File], separator: &str) -> String {
        let serialised_files: Vec<String> = files
            .iter()
            .filter_map(|f| match to_string(f) {
                Ok(s) => Some(GoogleStudioLLM::strip_xml_declaration(&s).to_string()),
                Err(e) => {
                    eprintln!("Error serializing Files: {:?}", e);
                    None
                }
            })
            .collect();

        serialised_files.join(separator)
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
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
    async fn identify_relevant_results(
        &self,
        context: &Context,
        search_results: &[SearchResult],
    ) -> Result<IdentifyResponse, IterativeSearchError>;
    async fn decide_continue(
        &self,
        context: &mut Context,
    ) -> Result<DecideResponse, IterativeSearchError>;
}

// Main system struct
pub struct IterativeSearchSystem<T: LLMOperations> {
    context: Context,
    repository: Repository,
    llm_ops: T,
    complete: bool,
}

impl<T: LLMOperations> IterativeSearchSystem<T> {
    pub fn new(context: Context, repository: Repository, llm_ops: T) -> Self {
        Self {
            context,
            repository,
            llm_ops,
            complete: false,
        }
    }

    fn context(&self) -> &Context {
        &self.context
    }

    pub async fn run(&mut self) -> Result<CodeSymbolImportantResponse, IterativeSearchError> {
        let mut count = 0;
        while self.complete == false && count < 3 {
            println!("===========");
            println!("run loop #{}", count);
            println!("===========");
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
                    .map(|r| format!("{:?}\n", r))
                    .collect::<Vec<String>>()
                    .join("\n")
            );

            let identify_results = self.identify(&search_results).await?;

            self.context.update_scatch_pad(&identify_results.scatch_pad);

            println!("Scratch pad: \n{}", self.context.scatch_pad());

            println!(
                "{}",
                identify_results
                    .item
                    .iter()
                    .map(|r| format!("{:?}\n", r))
                    .collect::<Vec<String>>()
                    .join("\n")
            );

            self.context.add_files(
                identify_results
                    .item
                    .iter()
                    .map(|r| File::new(r.path()))
                    .collect::<Vec<File>>(),
            );

            println!("Context::files: {:?}", self.context().files());

            let decision = self.decide().await?;

            println!("{:?}", decision);

            self.context.update_scatch_pad(decision.suggestions());

            self.complete = decision.complete();

            count += 1;
        }

        let symbols = self
            .context()
            .file_paths_as_strings()
            .iter()
            .map(|path| CodeSymbolWithThinking::from_path(path))
            .collect();

        let ordered_symbols = self
            .context()
            .file_paths_as_strings()
            .iter()
            .map(|path| CodeSymbolWithSteps::from_path(path))
            .collect();

        let response = CodeSymbolImportantResponse::new(symbols, ordered_symbols);

        Ok(response)
    }

    // this generates search queries
    async fn search(&self) -> Result<Vec<SearchQuery>, IterativeSearchError> {
        self.llm_ops.generate_search_query(self.context()).await
    }

    // identifies keywords to keep
    async fn identify(
        &mut self,
        search_results: &[SearchResult],
    ) -> Result<IdentifyResponse, IterativeSearchError> {
        self.llm_ops
            .identify_relevant_results(self.context(), search_results)
            .await
    }

    async fn decide(&mut self) -> Result<DecideResponse, IterativeSearchError> {
        self.llm_ops.decide_continue(&mut self.context).await
    }
}
