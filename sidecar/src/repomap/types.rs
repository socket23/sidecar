use std::collections::HashMap;
use std::path::PathBuf;
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

    pub fn try_parsing(fname: &str, ts_parsing: Arc<TSLanguageParsing>) {
        let lang = ts_parsing.detect_lang(fname);

        println!("Lang: {:?}", lang);

        let config = ts_parsing
            .for_lang(lang)
            .expect("language config to be present");

        let content = std::fs::read_to_string(fname).unwrap();

        let outline_string = config.generate_file_outline_str(content.as_bytes());
        println!("Outline: {:?}", outline_string);
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
