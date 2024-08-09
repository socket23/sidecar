use std::path::PathBuf;

use llm_client::{
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::{repo::iterator::RepositoryDirectory, repomap::tag::TagIndex};

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

// todo(zi): structure this based on available search tools
pub struct SearchQuery {
    query: String,
}

impl SearchQuery {
    pub fn new(query: String) -> Self {
        Self { query }
    }
}

struct SearchResult {
    files: Vec<File>,
}

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

// Main system struct
pub struct IterativeSearchSystem {
    query: IterativeSearchQuery,
}

impl IterativeSearchSystem {
    pub fn new(query: IterativeSearchQuery) -> Self {
        Self { query }
    }

    pub fn run(&mut self) {
        let mut count = 0;
        while count < 1 {
            println!("run loop #{}", count);
            let search_query = self.search();
            let search_result = self.query.repository.execute_search(search_query);
            self.identify(&search_result);
            if !self.decide() {
                break;
            }

            count += 1;
        }
    }

    // this generates the search_query based on context
    fn search(&self) -> SearchQuery {
        // construct LLM input for search

        // execute LLM call

        // execute_search (on repo)

        // Use self.context to generate a structured search query
        SearchQuery {
            query: String::new(),
        }
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
