use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::chunking::languages::TSLanguageParsing;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMap {
    root: PathBuf,
    // max_map_tokens: usize,
    // map_mul_no_files: usize,
    // max_context_window: Option<usize>,
    // tags_cache: HashMap<PathBuf, CachedTags>,
    // verbose: bool,
    queries_cache: HashMap<String, String>,
    package_path: String,
}

impl RepoMap {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            // max_map_tokens,
            // map_mul_no_files,
            // max_context_window,
            // tags_cache: HashMap::new(),
            // verbose,
            queries_cache: HashMap::new(),
            package_path: env!("CARGO_MANIFEST_DIR").to_string(),
        }
    }

    pub fn try_parsing(&self, fname: &str, ts_parsing: Arc<TSLanguageParsing>) {
        let path = PathBuf::from(&self.package_path).join(fname);

        if !path.exists() {
            eprintln!("Error: File not found: {}", path.display());
            return;
        }

        let config = match ts_parsing.for_file_path(fname) {
            Some(config) => config,
            None => {
                eprintln!("Error: Language configuration not found for: {}", fname);
                return;
            }
        };

        let content = match read_to_string(&path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file {}: {}", path.display(), e);
                return;
            }
        };

        let tree = config.get_tree_sitter_tree(content.as_bytes());

        if let Some(tree) = tree {
            // let root = tree.root_node();

            let tags = config.get_tags(content.as_bytes(), &tree, fname, fname);

            for tag in tags {
                println!("======\n{:?}\n======", tag);
            }
        }
        // let definitions
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CachedTags {
    mtime: std::time::SystemTime,
    data: Vec<Tag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag {
    pub rel_fname: String,
    pub fname: String,
    pub line: usize,
    pub name: String,
    pub kind: TagKind,
}

impl Tag {
    pub fn new(rel_fname: String, fname: String, line: usize, name: String, kind: TagKind) -> Self {
        Self {
            rel_fname,
            fname,
            line,
            name,
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagKind {
    Definition,
    Reference,
}
