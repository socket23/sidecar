use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use tokio::join;

use crate::{
    agentic::{
        symbol::identifier::LLMProperties,
        tool::{
            code_symbol::{
                important::CodeSymbolImportantResponse,
                repo_map_search::{RepoMapSearchBroker, RepoMapSearchQuery},
                types::CodeSymbolError,
            },
            errors::ToolError,
            file::file_finder::{ImportantFilesFinderBroker, ImportantFilesFinderQuery},
            input::ToolInput,
            output::ToolOutput,
            r#type::Tool,
        },
    },
    repomap::{tag::TagIndex, types::RepoMap},
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
        let request = match input {
            ToolInput::BigSearch(req) => req,
            _ => {
                return Err(ToolError::BigSearchError(
                    "Expected BigSearch input".to_string(),
                ))
            }
        };

        let root_directory = match request.root_directory() {
            Some(dir) => dir,
            None => {
                return Err(ToolError::BigSearchError(
                    "Root directory is required".to_string(),
                ))
            }
        };

        let start_time = std::time::Instant::now();

        let tree_broker = ImportantFilesFinderBroker::new(self.llm_client(), self.fail_over_llm());

        // could be parallelized?
        let (tree_string, _, _) =
            TreePrinter::to_string(Path::new(root_directory)).unwrap_or(("".to_string(), 0, 0));

        println!("{}", tree_string);

        let tree_input = ToolInput::ImportantFilesFinder(ImportantFilesFinderQuery::new(
            tree_string,
            request.user_query().to_string(),
            request.llm().clone(),
            request.provider().clone(),
            request.api_keys().clone(),
            root_directory.to_string(), // todo: this should be reponame
            request.root_request_id().to_string(),
        ));

        // could be parallelized?
        let tag_index = TagIndex::from_path(Path::new(root_directory)).await;
        let repo_map = RepoMap::new().with_map_tokens(1_000);
        let repo_map_string = repo_map
            .get_repo_map(&tag_index)
            .await
            .unwrap_or("".to_string());

        let repo_map_broker = RepoMapSearchBroker::new(self.llm_client(), self.fail_over_llm());
        let repo_map_input = ToolInput::RepoMapSearch(RepoMapSearchQuery::new(
            repo_map_string,
            request.user_query().to_string(),
            request.llm().clone(),
            request.provider().clone(),
            request.api_keys().clone(),
            request.root_directory().map(|d| d.to_string()),
            request.root_request_id().to_string(),
        ));

        // Run both invoke() methods in parallel
        let (tree_result, repo_map_result) = join!(
            tree_broker.invoke(tree_input),
            repo_map_broker.invoke(repo_map_input)
        );
        // Handle the results
        let tree_output: ToolOutput = tree_result?;
        let repo_map_output: ToolOutput = repo_map_result?;

        println!("tree_output: {:?}", tree_output);
        println!("repo_map_output: {:?}", repo_map_output);

        let elapsed_time = start_time.elapsed();
        println!("Elapsed time: {:?}", elapsed_time);

        let mut responses = Vec::new();

        match tree_output {
            ToolOutput::ImportantSymbols(important_symbols) => {
                println!("tree_output");
                important_symbols.print_symbol_count();
                responses.push(important_symbols);
            }
            _ => {
                // todo handle this
                println!("Unexpected output type for tree_output");
            }
        }

        println!("----------------------------------------");

        match repo_map_output {
            ToolOutput::RepoMapSearch(important_symbols) => {
                println!("repo_map_output");
                important_symbols.print_symbol_count();
                responses.push(important_symbols);
            }
            _ => {
                // todo handle this
                println!("Unexpected output type for repo_map_output");
            }
        }

        println!("----------------------------------------");

        let merged_output = CodeSymbolImportantResponse::merge(responses);

        println!("merged_output");
        merged_output.print_symbol_count();

        Ok(ToolOutput::BigSearch(merged_output))
    }
}
