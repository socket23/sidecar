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

    pub fn get_query(&mut self, lang: &str) -> String {
        let query_key = format!("tree-sitter-{}-tags", lang);

        if !self.queries_cache.contains_key(&query_key) {
            let path = self.construct_path_from_string(format!(
                "src/repomap/queries/tree-sitter-{}-tags.scm",
                lang
            ));
            let query = read_to_string(path).expect("Should have been able to read the file");
            self.queries_cache.insert(query_key.clone(), query);
        }

        self.queries_cache.get(&query_key).unwrap().clone()
    }

    fn construct_path_from_string(&self, path: String) -> PathBuf {
        PathBuf::from(&self.package_path).join(path)
    }

    // pub fn get_code(path: PathBuf) -> String {}
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
