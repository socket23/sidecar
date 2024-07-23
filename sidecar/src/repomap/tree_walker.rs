use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
};

use tree_sitter::{Node, Tree, TreeCursor};

use crate::chunking::languages::{TSLanguageConfig, TSLanguageParsing};

pub struct TreeWalker<'a> {
    scopes: Vec<HashSet<usize>>, // the starting lines of the nodes that span the line
    header: Vec<Vec<(usize, usize, usize)>>, // the size, start line, end line of the nodes that span the line
    nodes: Vec<Vec<Node<'a>>>,               // tree-sitter node requires lifetime parameter
    tree: Tree,
}

impl<'a> TreeWalker<'a> {
    pub fn new(tree: Tree, num_lines: usize) -> Self {
        Self {
            scopes: vec![HashSet::new(); num_lines],
            header: vec![Vec::new(); num_lines],
            nodes: vec![Vec::new(); num_lines],
            tree,
        }
    }

    fn walk_tree(&mut self, cursor: Node<'a>) {
        let mut cursor = cursor.walk();
        let start_line = cursor.node().start_position().row;
        let end_line = cursor.node().end_position().row;
        let size = end_line - start_line;

        self.nodes[start_line].push(cursor.node());

        // only assign headers to nodes that span more than one line
        // multiple nodes may share the same start line
        if size > 0 {
            self.header[start_line].push((size, start_line, end_line));
        }

        // for every line the node spans, add the position of its start line
        for i in start_line..=end_line {
            self.scopes[i].insert(start_line);
        }

        if cursor.goto_first_child() {
            loop {
                self.walk_tree(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    pub fn get_scopes(&self) -> &Vec<HashSet<usize>> {
        &self.scopes
    }

    pub fn get_headers(&self) -> &Vec<Vec<(usize, usize, usize)>> {
        &self.header
    }

    pub fn get_nodes(&self) -> &Vec<Vec<Node<'a>>> {
        &self.nodes
    }
}
