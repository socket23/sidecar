use std::path::Path;

use super::{
    javascript::javascript_language_config, rust::rust_language_config,
    typescript::typescript_language_config,
};

fn naive_chunker(buffer: &str, line_count: usize, overlap: usize) -> Vec<String> {
    let mut chunks: Vec<String> = vec![];
    let mut current_chunk = buffer
        .lines()
        .into_iter()
        .map(|line| line.to_owned())
        .collect::<Vec<_>>();
    let chunk_length = current_chunk.len();
    let mut start = 0;
    while start < chunk_length {
        let end = (start + line_count).min(chunk_length);
        let chunk = current_chunk[start..end].to_owned();
        chunks.push(chunk.join("\n"));
        start += line_count - overlap;
    }
    chunks
}

/// We are going to use tree-sitter to parse the code and get the chunks for the
/// code. we are going to use the algo sweep uses for tree-sitter
///
#[derive(Debug)]
pub struct TSLanguageConfig {
    /// A list of language names that can be processed by these scope queries
    /// e.g.: ["Typescript", "TSX"], ["Rust"]
    pub language_ids: &'static [&'static str],

    /// Extensions that can help classify the file: .rs, .rb, .cabal
    pub file_extensions: &'static [&'static str],

    /// tree-sitter grammar for this language
    pub grammar: fn() -> tree_sitter::Language,

    /// Namespaces defined by this language,
    /// E.g.: type namespace, variable namespace, function namespace
    pub namespaces: Vec<String>,
}

impl TSLanguageConfig {
    pub fn get_language(&self) -> Option<String> {
        self.language_ids.first().map(|s| s.to_string())
    }
}

pub struct TSLanguageParsing {
    configs: Vec<TSLanguageConfig>,
}

impl TSLanguageParsing {
    pub fn init() -> Self {
        Self {
            configs: vec![
                javascript_language_config(),
                typescript_language_config(),
                rust_language_config(),
            ],
        }
    }

    /// We will use this to chunk the file to pieces which can be used for
    /// searching
    pub async fn chunk_file(&self, file_path: &Path) -> Vec<Span> {
        let file_extension = file_path.extension();
        let file_content = tokio::fs::read_to_string(file_path).await.unwrap();
        if file_extension.is_none() {
            // We use naive chunker here which just splits on the number
            // of lines
            let chunks = naive_chunker(&file_content, 30, 15);
            let mut snippets: Vec<Span> = Default::default();
            chunks.iter().enumerate().for_each(|(i, _chunk)| {
                let start = i * 30;
                let end = (i + 1) * 30;
                snippets.push(Span::new(start, end, None));
            });
            return snippets;
        }
        // We try to find which language config we should use for this file
        let language_config_maybe = self.configs.iter().find(|config| {
            config
                .file_extensions
                .contains(&file_extension.unwrap().to_str().unwrap())
        });
        if let Some(language_config) = language_config_maybe {
            // We use tree-sitter to parse the file and get the chunks
            // for the file
            let language = language_config.grammar;
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(language()).unwrap();
            let buffer = std::fs::read_to_string(file_path).unwrap();
            let tree = parser.parse(&buffer, None).unwrap();
            let chunks = chunk_tree(&tree, language_config, 256, 30, &buffer);
            chunks
        } else {
            // use naive chunker here which just splits the file into parts
            let chunks = naive_chunker(&file_content, 30, 15);
            let mut snippets: Vec<Span> = Default::default();
            chunks.iter().enumerate().for_each(|(i, _chunk)| {
                let start = i * 30;
                let end = (i + 1) * 30;
                snippets.push(Span::new(start, end, None));
            });
            snippets
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub language: Option<String>,
}

impl Span {
    fn new(start: usize, end: usize, language: Option<String>) -> Self {
        Self {
            start,
            end,
            language,
        }
    }

    fn len(&self) -> usize {
        self.end - self.start
    }
}

fn chunk_node(
    mut node: tree_sitter::Node,
    language: &TSLanguageConfig,
    max_chars: usize,
) -> Vec<Span> {
    let mut chunks: Vec<Span> = vec![];
    let mut current_chunk = Span::new(node.start_byte(), node.end_byte(), language.get_language());
    let mut node_walker = node.walk();
    let current_node_children = node.children(&mut node_walker);
    for child in current_node_children {
        if child.end_byte() - child.start_byte() > max_chars {
            chunks.push(current_chunk.clone());
            current_chunk = Span::new(child.end_byte(), child.end_byte(), language.get_language());
            chunks.extend(chunk_node(child, language, max_chars));
        } else if child.end_byte() - child.start_byte() + current_chunk.len() > max_chars {
            chunks.push(current_chunk.clone());
            current_chunk = Span::new(
                child.start_byte(),
                child.end_byte(),
                language.get_language(),
            );
        } else {
            current_chunk.end = child.end_byte();
        }
    }
    chunks.push(current_chunk);
    chunks
}

/// We want to get back the non whitespace length of the string
fn non_whitespace_len(s: &str) -> usize {
    s.chars().filter(|c| !c.is_whitespace()).count()
}

fn get_line_number(byte_position: usize, split_lines: &[&str]) -> usize {
    let mut line_number = 0;
    let mut current_position = 0;
    for line in split_lines {
        if current_position + line.len() > byte_position {
            return line_number;
        }
        current_position += line.len();
        line_number += 1;
    }
    line_number
}

pub fn chunk_tree(
    tree: &tree_sitter::Tree,
    language: &TSLanguageConfig,
    max_characters_per_chunk: usize,
    coalesce: usize,
    buffer_content: &str,
) -> Vec<Span> {
    let mut chunks: Vec<Span> = vec![];
    let root_node = tree.root_node();
    chunks = chunk_node(root_node, language, max_characters_per_chunk);

    if chunks.len() == 0 {
        return Default::default();
    }
    if chunks.len() < 2 {
        return vec![Span::new(0, chunks[0].end, language.get_language())];
    }
    for (prev, curr) in chunks.to_vec().iter_mut().zip(chunks.iter_mut().skip(1)) {
        prev.end = curr.start;
    }

    let mut new_chunks: Vec<Span> = Default::default();
    let mut current_chunk = Span::new(0, 0, language.get_language());
    for chunk in chunks.iter() {
        current_chunk = Span::new(current_chunk.start, chunk.end, language.get_language());
        if non_whitespace_len(buffer_content[current_chunk.start..current_chunk.end].trim())
            > coalesce
        {
            new_chunks.push(current_chunk.clone());
            current_chunk = Span::new(chunk.end, chunk.end, language.get_language());
        }
    }

    if current_chunk.len() > 0 {
        new_chunks.push(current_chunk.clone());
    }

    let split_lines = buffer_content.split("\n").collect::<Vec<_>>();

    let mut line_chunks = new_chunks
        .iter()
        .map(|chunk| {
            let start_line = get_line_number(chunk.start, split_lines.as_slice());
            let end_line = get_line_number(chunk.end, split_lines.as_slice());
            Span::new(start_line, end_line, language.get_language())
        })
        .filter(|span| span.len() > 0)
        .collect::<Vec<Span>>();

    if line_chunks.len() > 1 && line_chunks.last().unwrap().len() < coalesce {
        let chunks_len = line_chunks.len();
        let last_chunk = line_chunks.last().unwrap().clone();
        let prev_chunk = line_chunks.get_mut(chunks_len - 2).unwrap();
        prev_chunk.end = last_chunk.end;
        line_chunks.pop();
    }

    line_chunks
}
