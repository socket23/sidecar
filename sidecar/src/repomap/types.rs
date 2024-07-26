use std::cmp::min;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::{stream, StreamExt};

use crate::chunking::languages::TSLanguageParsing;
use crate::repomap::tree_context::TreeContext;

use super::analyser::TagAnalyzer;
use super::error::RepoMapError;
use super::file::git::GitWalker;
use super::tag::{Tag, TagIndex};

pub struct RepoMap {
    git_walker: GitWalker,
    map_tokens: usize,
}

const REPOMAP_DEFAULT_TOKENS: usize = 1024;

impl RepoMap {
    pub fn new() -> Self {
        Self {
            git_walker: GitWalker {},
            map_tokens: REPOMAP_DEFAULT_TOKENS,
        }
    }

    pub fn with_map_tokens(mut self, map_tokens: usize) -> Self {
        self.map_tokens = map_tokens;
        self
    }

    async fn generate_tag_index(
        &self,
        files: HashMap<String, Vec<u8>>,
    ) -> Result<TagIndex, RepoMapError> {
        let mut tag_index = TagIndex::new();

        let ts_parsing = Arc::new(TSLanguageParsing::init());
        let _ = stream::iter(
            files
                .into_iter()
                .map(|(file, _)| (file, ts_parsing.clone())),
        )
        .map(|(file, ts_parsing)| async move {
            self.generate_tags_for_file(&file, ts_parsing)
                .await
                .map(|tags| (tags, file))
                .ok()
        })
        .buffer_unordered(10000)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter_map(|s| s)
        .for_each(|(tags, file)| {
            let file_ref = &file;
            tags.into_iter().for_each(|tag| {
                tag_index.add_tag(tag, &PathBuf::from(file_ref));
            });
        });

        tag_index.post_process_tags();

        Ok(tag_index)
    }

    pub async fn get_repo_map(&self, root: &Path) -> Result<String, RepoMapError> {
        let files = self.git_walker.read_files(root)?;

        let repomap = self.get_ranked_tags_map(files, self.map_tokens).await?;

        if repomap.is_empty() {
            return Err(RepoMapError::TreeGenerationError(
                "No tree generated".to_string(),
            ));
        }

        println!("Repomap: {}k tokens", self.get_token_count(&repomap) / 1024);

        Ok(repomap)
    }

    fn get_token_count(&self, tree: &str) -> usize {
        let chars = tree.chars().count();

        // https://platform.openai.com/tokenizer
        let token_per_char_ratio = 0.25;

        let token_estimate = (chars as f64 * token_per_char_ratio) as usize;

        token_estimate
    }

    fn find_best_tree(&self, ranked_tags: Vec<Tag>, max_map_tokens: usize) -> String {
        let num_tags = ranked_tags.len();
        println!("Initial conditions:");
        println!("  Number of tags: {}", num_tags);
        println!("  Max map tokens: {}", max_map_tokens);

        let mut lower_bound = 0;
        let mut upper_bound = num_tags;
        let mut best_tree = String::new();
        let mut best_tree_tokens = 0;
        let mut middle = min(max_map_tokens / 25, num_tags);
        let mut iteration = 0;

        while lower_bound <= upper_bound {
            iteration += 1;
            println!("\nIteration {}:", iteration);
            println!("  Bounds: [{}, {}]", lower_bound, upper_bound);
            println!("  Middle: {}", middle);

            // The clone here is very very expensive
            let tree = self.to_tree(&ranked_tags[..middle].to_vec());
            let num_tokens = self.get_token_count(&tree);

            println!("  Tree tokens: {}", num_tokens);

            if num_tokens < max_map_tokens && num_tokens > best_tree_tokens {
                println!("  New best tree found!");
                println!("    Previous best: {} tokens", best_tree_tokens);
                println!("    New best: {} tokens", num_tokens);
                best_tree.replace_range(.., &tree);
                best_tree_tokens = num_tokens;
            }

            if num_tokens < max_map_tokens {
                println!("  Increasing lower bound");
                lower_bound = middle + 1;
            } else {
                println!("  Decreasing upper bound");
                upper_bound = middle - 1;
            }

            middle = (lower_bound + upper_bound) / 2;

            println!("  Next middle: {}", middle);
        }

        println!("\nSearch completed:");
        println!("  Best tree tokens: {}", best_tree_tokens);
        println!("  Final bounds: [{}, {}]", lower_bound, upper_bound);

        best_tree
    }

    pub async fn get_ranked_tags_map(
        &self,
        files: HashMap<String, Vec<u8>>,
        max_map_tokens: usize,
    ) -> Result<String, RepoMapError> {
        println!("[TagIndex] Generating tags from {} files...", files.len());
        let tag_index = self.generate_tag_index(files).await?;

        let mut analyser = TagAnalyzer::new(tag_index);

        println!("[Analyser] Ranking tags...");
        let ranked_tags = analyser.get_ranked_tags().clone();
        println!("[Analyser] tags::len({})", ranked_tags.len());

        println!("[Tree] Finding best tree...");
        let tree = self.find_best_tree(ranked_tags, max_map_tokens);

        Ok(tree)
    }

    fn to_tree(&self, tags: &Vec<Tag>) -> String {
        let mut tags = tags.clone();
        tags.sort_by(|a, b| a.rel_fname.cmp(&b.rel_fname));
        tags.push(Tag::dummy());

        let mut output = String::new();

        let mut cur_fname = "";
        let mut cur_abs_fname = "";

        let mut lois: Option<Vec<usize>> = None;

        for tag in &tags {
            let this_rel_fname = tag.rel_fname.to_str().expect("to_str to work for path");
            let fname = tag.fname.to_str().expect("to_str to work for path");

            // check whether filename has changed, including first iteration
            if this_rel_fname != cur_fname {
                // take() resets the lois to None, inner_lois may be used as value for render_tree
                if let Some(inner_lois) = lois.take() {
                    output.push('\n');
                    output.push_str(&cur_fname);
                    output.push_str(":\n");
                    let file_content = std::fs::read(&cur_abs_fname);
                    if let Err(_) = file_content {
                        continue;
                    }
                    let file_content = file_content.expect("file_content to be present");
                    // read the file content and keep track of it
                    output.push_str(&self.render_tree(
                        &cur_abs_fname,
                        &cur_fname,
                        &inner_lois,
                        &file_content,
                    ));
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

    fn render_tree(
        &self,
        abs_fname: &str,
        _rel_fname: &str,
        lois: &Vec<usize>,
        file_content: &Vec<u8>,
    ) -> String {
        let mut code = String::from_utf8_lossy(file_content.as_slice()).to_string();
        if !code.ends_with('\n') {
            code.push('\n');
        }

        let ts_parsing = TSLanguageParsing::init();
        let config = ts_parsing.for_file_path(abs_fname).unwrap().clone();

        let tree = config.get_tree_sitter_tree(code.as_bytes()).unwrap();

        let root_node = tree.root_node();

        let cursor = root_node.walk();

        // todo - consider using rel_fname
        let mut context = TreeContext::new(code, abs_fname.to_owned());

        context.init(cursor);

        context.add_lois(lois.clone());

        context.add_context();

        context.format()
    }

    fn get_rel_fname(&self, fname: &PathBuf) -> PathBuf {
        let self_root = env!("CARGO_MANIFEST_DIR").to_string();
        fname
            .strip_prefix(&self_root)
            .unwrap_or(fname)
            .to_path_buf()
    }

    async fn generate_tags_for_file(
        &self,
        fname: &str,
        ts_parsing: Arc<TSLanguageParsing>,
    ) -> Result<Vec<Tag>, RepoMapError> {
        let rel_path = self.get_rel_fname(&PathBuf::from(fname));
        let config = ts_parsing.for_file_path(fname).ok_or_else(|| {
            RepoMapError::ParseError(format!("Language configuration not found for: {}", fname,))
        });
        let content = tokio::fs::read(fname).await;
        if let Err(_) = content {
            return Err(RepoMapError::IoError);
        }
        let content = content.expect("if let Err to hold");
        if let Ok(config) = config {
            let tags = config
                .get_tags(&PathBuf::from(fname), &rel_path, content)
                .await;
            Ok(tags)
        } else {
            Ok(vec![])
        }
    }
}
