use llm_client::clients::types::LLMType;
use llm_client::tokenizer::tokenizer::LLMTokenizer;
use std::cmp::min;
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
    map_tokens: usize,
}

const REPOMAP_DEFAULT_TOKENS: usize = 1024;

impl RepoMap {
    pub fn new(fs: Box<dyn FileSystem>) -> Self {
        Self {
            fs,
            map_tokens: REPOMAP_DEFAULT_TOKENS,
        }
    }

    pub fn with_map_tokens(mut self, map_tokens: usize) -> Self {
        self.map_tokens = map_tokens;
        self
    }

    fn generate_tag_index(&self, files: &[PathBuf]) -> Result<TagIndex, RepoMapError> {
        let mut tag_index = TagIndex::new();

        let ts_parsing = Arc::new(TSLanguageParsing::init());
        for file in files {
            self.process_file(&file, &ts_parsing, &mut tag_index)?;
        }

        tag_index.post_process_tags();

        Ok(tag_index)
    }

    pub fn get_repo_map(&self, root: &Path) -> Result<String, RepoMapError> {
        let files = self.fs.get_files(root)?;
        let file_listing = self.get_ranked_tags_map(files, self.map_tokens)?;

        let mut repo_content = String::new();
        repo_content.push_str(&file_listing);

        println!("{}", repo_content);

        Ok(repo_content)
    }

    fn get_token_count(&self, tree: &str) -> usize {
        let tokenizer = LLMTokenizer::new().unwrap();
        let llm_type = LLMType::Gpt4O; // default
        let token_count = tokenizer
            .count_tokens_using_tokenizer(&llm_type, &tree)
            .unwrap();

        println!("get_token_count::({})", token_count);

        token_count
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

    pub fn get_ranked_tags_map(
        &self,
        files: Vec<PathBuf>,
        max_map_tokens: usize,
    ) -> Result<String, RepoMapError> {
        let tag_index = self.generate_tag_index(&files)?;

        let mut analyser = TagAnalyzer::new(tag_index);

        let ranked_tags = analyser.get_ranked_tags().clone();

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

    fn render_tree(&self, abs_fname: &str, _rel_fname: &str, lois: &Vec<usize>) -> String {
        let mut code = self.fs.read_file(Path::new(abs_fname)).unwrap();

        if !code.ends_with('\n') {
            code.push('\n');
        }

        let ts_parsing = TSLanguageParsing::init();
        let config = ts_parsing.for_file_path(abs_fname).unwrap().clone();

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
            });

        if let Ok(config) = config {
            let tags = config.get_tags(fname, &rel_path);

            tags.into_iter().for_each(|tag| {
                tag_index.add_tag(tag, rel_path.clone());
            });
        }

        Ok(())
    }
}
