//! Generates the repo map for a given sub-directory and for all the files
//! which are present inside the sub-directory taking into considering the .gitignore
//! and other smart filters which we can apply

use std::path::Path;

use crate::{
    agentic::tool::{
        errors::ToolError, input::ToolInput, lsp::list_files::list_files, output::ToolOutput,
        r#type::Tool,
    },
    repomap::{tag::TagIndex, types::RepoMap},
};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct RepoMapGeneratorRequest {
    directory_path: String,
    token_count: usize,
}

impl RepoMapGeneratorRequest {
    pub fn new(directory_path: String, token_count: usize) -> Self {
        Self {
            directory_path,
            token_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RepoMapGeneratorResponse {
    repo_map: String,
}

impl RepoMapGeneratorResponse {
    pub fn new(repo_map: String) -> Self {
        Self { repo_map }
    }

    pub fn repo_map(&self) -> &str {
        &self.repo_map
    }
}

pub struct RepoMapGeneratorClient {}

impl RepoMapGeneratorClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Tool for RepoMapGeneratorClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_repo_map_generation()?;
        let token_count = context.token_count;
        let directory_path = Path::new(&context.directory_path);

        // give a large limit to the number of files which we are generating over here
        let files_in_directory = list_files(directory_path, true, 10_000)
            .0
            .into_iter()
            .filter_map(|inside_path| {
                if inside_path.is_dir() {
                    None
                } else {
                    Some(inside_path)
                }
            })
            .map(|file_path| file_path.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        let tag_index = TagIndex::from_files(directory_path, files_in_directory).await;

        let repo_map = RepoMap::new().with_map_tokens(token_count);

        let repo_map_string = repo_map.get_repo_map(&tag_index).await;
        repo_map_string
            .map_err(|e| ToolError::RepoMapError(e))
            .map(|output| {
                ToolOutput::repo_map_generation_reponse(RepoMapGeneratorResponse::new(output))
            })
    }

    fn tool_description(&self) -> String {
        r#""#.to_owned()
    }

    fn tool_input_format(&self) -> String {
        r#""#.to_owned()
    }
}
