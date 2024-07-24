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

    pub fn generate(&self, root: &Path) -> Result<Vec<Tag>, RepoMapError> {
        let files = self.fs.get_files(root)?;
        let mut tag_index = TagIndex::new();

        let ts_parsing = Arc::new(TSLanguageParsing::init());
        for file in files {
            self.process_file(&file, &ts_parsing, &mut tag_index)?;
        }

        self.post_process_tags(&mut tag_index);

        let mut analyser = TagAnalyzer::new(tag_index);

        let ranked_tags = analyser.get_ranked_tags().clone();

        // analyser.debug_print_ranked_tags();

        let tree_string = self.to_tree(&ranked_tags);

        println!("{}", tree_string);

        Ok(ranked_tags)
    }

    fn to_tree(&self, tags: &Vec<Tag>) -> String {
        let mut tags = tags.clone();
        tags.sort_by(|a, b| a.rel_fname.cmp(&b.rel_fname));
        tags.truncate(3);

        let mut output = String::new();

        let mut cur_fname = "";
        let mut cur_abs_fname = "";

        let mut lois: Option<Vec<usize>> = None;

        for tag in &tags {
            let this_rel_fname = tag.rel_fname.to_str().unwrap();

            // check whether filename has changed, including first iteration
            if this_rel_fname != cur_fname {
                // take() resets the lois to None, inner_lois may be used as value for render_tree
                if let Some(inner_lois) = lois.take() {
                    output.push('\n');
                    output.push_str(&cur_fname);
                    output.push_str(":\n");
                    output.push_str(&self.render_tree(&cur_abs_fname, &cur_fname, &inner_lois));
                } else if !cur_fname.is_empty() {
                    output.push('\n');
                    output.push_str(&cur_fname);
                    output.push('\n');
                }

                lois = Some(Vec::new());
                cur_abs_fname = tag.fname.to_str().unwrap();
                cur_fname = this_rel_fname;
            }

            // as_mut() is critical here as we want to mutate the original lois
            if let Some(lois) = lois.as_mut() {
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

    fn render_tree(&self, abs_fname: &str, rel_fname: &str, lois: &Vec<usize>) -> String {
        let mut code = self.fs.read_file(Path::new(abs_fname)).unwrap();

        if !code.ends_with('\n') {
            code.push('\n');
        }

        let ts_parsing = TSLanguageParsing::init();
        let config = ts_parsing.for_file_path(abs_fname).unwrap().clone();
        let lines: Vec<String> = code.split('\n').map(|s| s.to_string()).collect();
        let num_lines = lines.len() + 1;

        let tree = config.get_tree_sitter_tree(code.as_bytes()).unwrap();

        let root_node = tree.root_node();

        let cursor = root_node.walk();

        // todo - consider using rel_fname
        let mut context = TreeContext::new(code);
        context.init(cursor);

        context.add_lois(lois.clone());

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
