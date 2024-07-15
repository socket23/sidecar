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

    pub fn get_ranked_tags(
        &self,
        chat_fnames: &[PathBuf],
        other_fnames: &[PathBuf],
        ts_parsing: Arc<TSLanguageParsing>,
    ) {
        // set of file paths where the tag is defined
        // good for question "In which files is tag X defined?"
        let mut defines: HashMap<String, HashSet<PathBuf>> = HashMap::new();

        // allows duplicates to accommodate multiple references to the same definition
        let mut references: HashMap<String, Vec<PathBuf>> = HashMap::new();

        // map of (file path, tag name) to tag
        // good for question "What are the details of tag X in file Y?"
        let mut definitions: HashMap<(PathBuf, String), HashSet<Tag>> = HashMap::new();

        // TODO: implement personalization
        let mut personalization: HashMap<String, f64> = HashMap::new();

        let mut fnames: HashSet<PathBuf> = chat_fnames.iter().cloned().collect();
        fnames.extend(other_fnames.iter().cloned());

        let fnames: Vec<PathBuf> = fnames.into_iter().collect();

        for fname in &fnames {
            if !fname.exists() {
                eprintln!("Error: File not found: {}", fname.display());
                continue;
            }

            let rel_path = self.get_rel_fname(&fname);

            let config = match ts_parsing.for_file_path(fname.to_str().unwrap()) {
                Some(config) => config,
                None => {
                    eprintln!(
                        "Error: Language configuration not found for: {}",
                        fname.display()
                    );
                    continue;
                }
            };

            let tags = config.get_tags(fname, &rel_path);

            for tag in tags {
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
        }

        // if references are empty, use defines as references
        if references.is_empty() {
            references = defines
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().cloned().collect::<Vec<PathBuf>>()))
                .collect();
        }

        println!("==========Defines==========");
        for (key, set) in &defines {
            println!("Key {}, Set: {:?}", key, set);
        }

        println!("==========Definitions==========");
        for ((pathbuf, tag_name), set) in &definitions {
            println!("Key {:?}, Set: {:?}", (pathbuf, tag_name), set);
        }

        println!("==========References==========");
        for (tag_name, paths) in references {
            println!("Tag: {}, Paths: {:?}", tag_name, paths);
        }
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
