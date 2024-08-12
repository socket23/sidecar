use walkdir::WalkDir;

use std::{fs::read_to_string, path::PathBuf};

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
    pub fn find_file(&self, target: &str) -> Option<PathBuf> {
        WalkDir::new(&self.root)
            .into_iter()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy() == target)
            .map(|e| e.path().to_owned())
    }

    pub fn execute_search(&self, search_query: &SearchQuery) -> Vec<SearchResult> {
        // Implement repository search logic
        println!("repository::execute_search::query: {:?}", search_query);

        match search_query.tool {
            SearchToolType::File => {
                println!(
                    "repository::execute_search::query::SearchToolType::File, searching for {}",
                    search_query.query
                );

                let tags_in_file = self.tag_index.search_definitions_flattened(
                    &search_query.query,
                    false,
                    SearchMode::FilePath,
                );

                match tags_in_file.is_empty() {
                    true => {
                        println!("No tags for file: {}", search_query.query);

                        let file = self.find_file(&search_query.query);

                        println!(
                            "repository::execute_search::query::SearchToolType::File::file: {:?}",
                            file
                        );

                        if let Some(path) = file {
                            println!(
                                "repository::execute_search::query::SearchToolType::File::Some(path): {:?}",
                                path
                            );
                            let contents = match read_to_string(&path) {
                                Ok(content) => content,
                                Err(error) => {
                                    eprintln!("Error reading file: {}", error);
                                    "".to_owned()
                                }
                            };

                            vec![SearchResult::new(
                                path,
                                &search_query.thinking, // consider...
                                &contents,
                            )]
                        } else {
                            vec![SearchResult::new(
                                PathBuf::from("".to_string()),
                                &search_query.thinking,
                                "",
                            )]
                        }
                    }
                    false => {
                        println!("Tags found for file: {}", tags_in_file.len());

                        let search_results = tags_in_file
                            .iter()
                            .map(|t| {
                                let thinking_message =
                                    format!("This file contains a {:?} named {}", t.kind, t.name);
                                SearchResult::new(t.fname.to_owned(), &thinking_message, &t.name)
                            })
                            .collect::<Vec<SearchResult>>();

                        search_results
                    }
                }
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
                    .map(|t| {
                        let thinking_message =
                            format!("This file contains a {:?} named {}", t.kind, t.name);
                        SearchResult::new(t.fname.to_owned(), &thinking_message, &t.name)
                    })
                    .collect()
            }
        }
    }
}
