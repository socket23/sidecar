use walkdir::WalkDir;

use std::path::PathBuf;

use crate::{
    agentic::tool::search::exp::SearchToolType,
    repomap::tag::{SearchMode, TagIndex},
};

use super::exp::{SearchQuery, SearchResult};

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
    pub fn find_file(&self, target: &str) -> Option<String> {
        WalkDir::new(&self.root)
            .into_iter()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy() == target)
            .map(|e| e.path().to_string_lossy().into_owned())
    }

    pub fn execute_search(&self, search_query: &SearchQuery) -> Vec<SearchResult> {
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
