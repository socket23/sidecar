use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMap {
    root: PathBuf,
    // max_map_tokens: usize,
    // map_mul_no_files: usize,
    // max_context_window: Option<usize>,
    // tags_cache: HashMap<PathBuf, CachedTags>,
    // verbose: bool,
    queries_cache: HashMap<String, String>,
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
        }
    }

    pub fn get_query(&mut self, lang: &str) -> String {
        let query_key = format!("tree-sitter-{}-tags", lang);

        self.queries_cache
            .entry(query_key.clone())
            .or_insert_with(|| {
                let package_path = env!("CARGO_MANIFEST_DIR");
                let path = PathBuf::from(package_path)
                    .join("src")
                    .join("repomap")
                    .join("queries")
                    .join(format!("tree-sitter-{}-tags.scm", lang));

                read_to_string(path).expect("Should have been able to read the file")
            })
            .clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CachedTags {
    mtime: std::time::SystemTime,
    data: Vec<Tag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag {
    rel_fname: PathBuf,
    fname: PathBuf,
    line: usize,
    name: String,
    kind: TagKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagKind {
    Definition,
    Reference,
}
