use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::chunking::languages::TSLanguageParsing;
use crate::repomap::tree_context::TreeContext;

use super::analyser::TagAnalyzer;
use super::error::RepoMapError;
use super::files::FileSystem;
use super::tag::{Tag, TagIndex};

pub struct RepoMap {
    fs: Box<dyn FileSystem>,
}

impl RepoMap {
    pub fn new(fs: Box<dyn FileSystem>) -> Self {
        Self { fs }
    }

    pub fn generate(&self, root: &Path) -> Result<Vec<HashSet<Tag>>, RepoMapError> {
        let files = self.fs.get_files(root)?;
        let mut tag_index = TagIndex::new();

        let ts_parsing = Arc::new(TSLanguageParsing::init());
        for file in files {
            self.process_file(&file, &ts_parsing, &mut tag_index)?;
        }

        self.post_process_tags(&mut tag_index);

        let mut analyser = TagAnalyzer::new(tag_index);

        let ranked_tags = analyser.get_ranked_tags();

        let tree_string = self.to_tree(ranked_tags.clone());

        println!("{}", tree_string);

        Ok(ranked_tags.into_iter().map(|set| set.clone()).collect())
    }

    fn to_tree(&self, tags: Vec<&HashSet<Tag>>) -> String {
        let mut output = String::new();

        let mut cur_fname = "";
        let mut cur_abs_fname = "";

        let mut lois: Vec<usize> = vec![];

        for (i, tag_set) in tags.iter().enumerate() {
            println!("Number of tags in tag_set #{}: {}", i, tag_set.len());
            // there should only be one tag per file
            let tag = tag_set.iter().next().unwrap();
            let this_rel_fname = tag.rel_fname.to_str().unwrap();

            if this_rel_fname != cur_fname {
                if !lois.is_empty() {
                    output.push_str("\n");
                    output.push_str(&cur_fname);
                    // todo: output.push_str(render_tree(cur_abs_fname, cur_fname, lois));
                } else if !cur_fname.is_empty() {
                    output.push_str(&format!("\n{}\n", cur_fname));
                }

                lois = vec![];
                cur_abs_fname = tag.fname.to_str().unwrap();
                cur_fname = this_rel_fname;
            }

            if !lois.is_empty() {
                lois.push(tag.line);
            }
        }

        output = output
            .lines()
            .map(|line| {
                if line.len() > 100 {
                    line[..100].to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        output.push('\n');

        output
    }

    fn render_tree(&self, abs_fname: &str, rel_fname: &str, lois: Vec<usize>) -> String {
        println!("Rendering tree for abs_fname: {}", abs_fname);

        let mut code = self.fs.read_file(Path::new(abs_fname)).unwrap();

        if !code.ends_with('\n') {
            code.push('\n');
        }

        // todo - consider using rel_fname
        let mut context = TreeContext::new(abs_fname.to_string(), code);

        context.add_lois(lois);
        context.add_context();

        context.format()
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
