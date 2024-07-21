use crate::chunking::languages::TSLanguageParsing;
use std::collections::HashSet;
use thiserror::Error;
use tree_sitter::{Node, Tree, TreeCursor};

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

pub struct TreeContext<'a> {
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
    nodes: Vec<Vec<&'a Node<'a>>>, // tree-sitter node requires lifetime parameter
    num_lines: usize,
    tree: Tree,
    output: Vec<String>,
}

impl<'a> TreeContext<'a> {
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

        let root_node = &tree.root_node();

        // iterate through nodes
        // for each node, get start and end positions
        // add to nodes

        let mut cursor = root_node.walk();

        TreeContext::walk_tree(&mut cursor);

        // // Traverse child nodes
        // for child in root_node.children(&mut cursor) {
        //     TreeContext::walk_tree(child);
        // }

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
            tree: tree.clone(),
            output: vec![],
        })
    }

    fn print_node_at_cursor(cursor: &TreeCursor) {
        println!("Node type: {}", cursor.node().kind());
        println!("Node field_name: {:?}", cursor.field_name());
    }

    fn walk_tree(cursor: &mut TreeCursor) {
        // Create a cursor for traversing child nodes
        println!("Starting node:");
        TreeContext::print_node_at_cursor(&cursor);

        if !cursor.goto_first_child() {
            println!("No first child");
            return;
        }

        println!("First child");
        TreeContext::print_node_at_cursor(&cursor);

        let mut sibling_index = 0;
        while cursor.goto_next_sibling() {
            println!("sibling {sibling_index}");
            TreeContext::print_node_at_cursor(&cursor);
            sibling_index += 1;
        }

        println!("No next sibling");
    }

    // fn walk_tree(&mut self, node: Node<'a>) {
    //     let start = node.start_position();
    //     let end = node.end_position();

    //     let start_line = start.row;
    //     let start_col = start.column;
    //     let end_line = end.row;
    //     let end_col = end.column;

    //     let size = end_line - start_line;
    //     self.nodes[start_line].push(&node);
    // }

    // pub fn print_tree(&'a mut self) {
    //     let root_node = self.tree.root_node();
    //     self.walk_tree(root_node);
    // }

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
