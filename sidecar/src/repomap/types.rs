use std::collections::{HashMap, HashSet};
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

    fn get_rel_fname(&self, fname: &PathBuf) -> PathBuf {
        fname
            .strip_prefix(&self.root)
            .unwrap_or(fname)
            .to_path_buf()
    }

    pub fn try_repomap(&self, fname: &PathBuf, ts_parsing: Arc<TSLanguageParsing>) {
        let path = PathBuf::from(&self.package_path).join(fname);

        println!("path: {:?}", path);

        if !path.exists() {
            eprintln!("Error: File not found: {}", path.display());
            return;
        }

        let config = match ts_parsing.for_file_path(fname.to_str().unwrap()) {
            Some(config) => config,
            None => {
                eprintln!(
                    "Error: Language configuration not found for: {}",
                    fname.display()
                );
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

        let tree = match config.get_tree_sitter_tree(content.as_bytes()) {
            Some(tree) => tree,
            None => {
                eprintln!(
                    "Error: Failed to get tree-sitter tree for: {}",
                    path.display()
                );
                return;
            }
        };

        let rel_path = self.get_rel_fname(fname);

        let mut defines: HashMap<String, HashSet<String>> = HashMap::new();
        let mut references: HashMap<String, Vec<String>> = HashMap::new();
        let mut definitions: HashMap<(String, String), HashSet<Tag>> = HashMap::new();
        let mut personalization: HashMap<String, f64> = HashMap::new();

        let tags = config.get_tags(content.as_bytes(), &tree, fname, fname);

        for tag in tags {
            println!("======\n{:?}\n======", tag);

            let rel_path = rel_path.to_str().unwrap().to_string();
            match tag.kind {
                TagKind::Definition => {
                    defines
                        .entry(tag.name.clone())
                        .or_default()
                        .insert(rel_path.clone());
                    definitions
                        .entry((rel_path.clone(), tag.name.clone()))
                        .or_default()
                        .insert(tag);
                }
                TagKind::Reference => {
                    references
                        .entry(tag.name.clone())
                        .or_default()
                        .push(rel_path.clone());
                }
            }
        }

        println!("defines: {:?}", defines);
        println!("references: {:?}", references);
        println!("definitions: {:?}", definitions);

        // for tag in tags:
        // if tag.kind == "def":
        //     defines[tag.name].add(rel_fname)
        //     key = (rel_fname, tag.name)
        //     definitions[key].add(tag)

        // if tag.kind == "ref":
        //     references[tag.name].append(rel_fname)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CachedTags {
    mtime: std::time::SystemTime,
    data: Vec<Tag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag {
    pub rel_fname: PathBuf,
    pub fname: PathBuf,
    pub line: usize,
    pub name: String,
    pub kind: TagKind,
}

impl Tag {
    pub fn new(
        rel_fname: PathBuf,
        fname: PathBuf,
        line: usize,
        name: String,
        kind: TagKind,
    ) -> Self {
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
