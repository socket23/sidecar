use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::chunking::languages::TSLanguageParsing;

use super::analyser::TagAnalyzer;
use super::error::RepoMapError;
use super::files::FileSystem;
use super::tag::TagIndex;

pub struct RepoMap {
    fs: Box<dyn FileSystem>,
}

impl RepoMap {
    pub fn new(fs: Box<dyn FileSystem>) -> Self {
        Self { fs }
    }

    pub fn generate(&self, root: &Path) -> Result<bool, RepoMapError> {
        let files = self.fs.get_files(root)?;
        let mut tag_index = TagIndex::new();

        let ts_parsing = Arc::new(TSLanguageParsing::init());
        for file in files {
            self.process_file(&file, &ts_parsing, &mut tag_index)?;
        }

        self.post_process_tags(&mut tag_index);

        let mut analyser = TagAnalyzer::new(tag_index);

        let ranked_tags = analyser.get_ranked_tags();

        for tag_set in ranked_tags {
            for tag in tag_set {
                println!("{}", tag.name);
            }
            println!("---");
        }

        Ok(true)
    }

    fn post_process_tags(&self, tag_index: &mut TagIndex) {
        tag_index.process_empty_references();
        tag_index.process_common_tags();
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
}
