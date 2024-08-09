use std::path::PathBuf;

use crate::repomap::tag::TagIndex;

#[derive(Debug, Clone)]
pub struct Context {
    files: Vec<File>,
    user_query: String,
    thoughts: String,
}

impl Context {
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

// Main system struct
pub struct IterativeSearchSystem {
    context: Context,
    repository: Repository,
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

impl IterativeSearchSystem {
    pub fn new(user_query: String, repository: Repository) -> Self {
        Self {
            context: Context {
                files: Vec::new(),
                user_query,
                thoughts: String::new(),
            },
            repository,
        }
    }

    pub fn run(&mut self) {
        let mut count = 0;
        while count < 1 {
            println!("run loop #{}", count);
            let search_query = self.search();
            let search_result = self.repository.execute_search(search_query);
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
