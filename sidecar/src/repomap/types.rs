use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::chunking::languages::TSLanguageParsing;

use super::error::RepoMapError;
use super::files::FileSystem;
use super::tag::TagIndex;

// #[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMap {
    fs: Box<dyn FileSystem>,
    // root: PathBuf,
    // max_map_tokens: usize,
    // map_mul_no_files: usize,
    // max_context_window: Option<usize>,
    // tags_cache: HashMap<PathBuf, CachedTags>,
    // verbose: bool,
    // queries_cache: HashMap<String, String>,
    // package_path: String,
}

impl RepoMap {
    pub fn new(fs: Box<dyn FileSystem>) -> Self {
        Self {
            fs,
            // root,
            // max_map_tokens,
            // map_mul_no_files,
            // max_context_window,
            // tags_cache: HashMap::new(),
            // verbose,
            // queries_cache: HashMap::new(),
            // package_path: env!("CARGO_MANIFEST_DIR").to_string(),
        }
    }

    pub fn generate(&self, root: &Path) -> Result<bool, RepoMapError> {
        let files = self.fs.get_files(root)?;
        let mut tag_index = TagIndex::new();

        let ts_parsing = Arc::new(TSLanguageParsing::init());
        for file in files {
            self.process_file(&file, &ts_parsing, &mut tag_index)?;
        }
        tag_index.process_empty_references();
        tag_index.process_common_tags();

        tag_index.debug_print();

        Ok(true)
    }

    fn get_rel_fname(&self, fname: &PathBuf) -> PathBuf {
        let self_root = env!("CARGO_MANIFEST_DIR").to_string();
        fname
            .strip_prefix(&self_root)
            .unwrap_or(fname)
            .to_path_buf()
    }

    fn process_file(
        &self,
        fname: &PathBuf,
        ts_parsing: &Arc<TSLanguageParsing>,
        tag_index: &mut TagIndex,
    ) -> Result<(), RepoMapError> {
        let rel_path = self.get_rel_fname(fname);
        let config = ts_parsing
            .for_file_path(fname.to_str().unwrap())
            .ok_or_else(|| {
                RepoMapError::ParseError(format!(
                    "Language configuration not found for: {}",
                    fname.display()
                ))
            })?;

        let tags = config.get_tags(fname, &rel_path);

        for tag in tags {
            tag_index.add_tag(tag, rel_path.clone());
        }

        Ok(())
    }

    pub fn get_ranked_tags(
        &self,
        chat_fnames: &[PathBuf],
        other_fnames: &[PathBuf],
        ts_parsing: Arc<TSLanguageParsing>,
        tag_index: &mut TagIndex,
        // mentioned_fnames: Option<&[PathBuf]>,
        // mentioned_idents: Option<&[String]>,
    ) {
        // TODO: implement personalization
        // let mut personalization: HashMap<String, f64> = HashMap::new();

        let fnames: HashSet<PathBuf> = chat_fnames
            .iter()
            .chain(other_fnames.iter())
            .cloned()
            .collect();

        for fname in &fnames {
            if let Err(e) = self.process_file(fname, &ts_parsing, tag_index) {
                eprintln!("Error processing file {}: {}", fname.display(), e);
            }
        }

        // if references are empty, use defines as references
        tag_index.process_empty_references();
        tag_index.process_common_tags();
    }
}
