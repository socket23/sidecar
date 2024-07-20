use crate::chunking::languages::{TSLanguageConfig, TSLanguageParsing};
use std::{borrow::Cow, collections::HashSet, sync::Arc};
use thiserror::Error;
use tree_sitter::{Node, Tree};

#[derive(Debug, Error)]
pub enum TreePrinterError {
    #[error("No language configuration found for file: {0}")]
    MissingConfig(String),
    #[error("Failed to parse tree for file: {0}")]
    ParseError(String),
    #[error("Invalid tree structure: {0}")]
    InvalidTree(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

struct TreeContext {
    filename: String,
    code: String,
    line_number: bool,
    parent_context: bool,
    child_context: bool,
    last_line: bool,
    margin: usize,
    mark_lois: bool,
    header_max: usize,
    show_top_of_file_parent_scope: bool,
    loi_pad: usize,
    scopes: Vec<HashSet<usize>>,
    header: Vec<Vec<(usize, usize, usize)>>,
    nodes: Vec<Vec<Node<'static>>>,
    num_lines: usize,
    tree: Tree,
    // ts_config: Arc<TSLanguageConfig>,
}

impl TreeContext {
    pub fn new(
        filename: String,
        code: String,
        ts_parser: &TSLanguageParsing,
    ) -> Result<Self, TreePrinterError> {
        let ts_config = ts_parser
            .for_file_path(&filename)
            .ok_or(TreePrinterError::MissingConfig(filename.clone()))?;

        let tree = ts_config
            .get_tree_sitter_tree(code.as_bytes())
            .ok_or(TreePrinterError::ParseError(filename.clone()))?;

        let num_lines = code.lines().count();

        Ok(Self {
            filename,
            code,
            line_number: false,
            parent_context: true,
            child_context: true,
            last_line: true,
            margin: 3,
            mark_lois: true,
            header_max: 10,
            show_top_of_file_parent_scope: false,
            loi_pad: 1,
            scopes: vec![HashSet::new(); num_lines],
            header: vec![Vec::new(); num_lines],
            nodes: vec![Vec::new(); num_lines],
            num_lines,
            tree,
            // ts_config: Arc::new(ts_config.clone()),
        })
    }

    // get lines count - dropped, this is unnecessary

    // initialise output lines HashMap

    // initialise scopes, headers, nodes

    // walk tree

    // add lines of interest (lois)

    // add context()

    // format
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_printer() -> Result<(), TreePrinterError> {
        let ts_parser = TSLanguageParsing::init();
        let tree_context = TreeContext::new("test.ts".to_string(), "".to_string(), &ts_parser)?;
        Ok(())
    }
}
