use std::collections::HashSet;

use tree_sitter::Node;

use crate::chunking::languages::{TSLanguageConfig, TSLanguageParsing};

use super::tree_printer::TreePrinter;

pub struct TreeContext<'a> {
    filename: String,
    code: String,
    parent_context: bool,
    child_context: bool,
    last_line: bool,
    margin: usize,
    mark_lois: bool,
    header_max: usize,
    show_top_of_file_parent_scope: bool,
    loi_pad: usize,
    output: Vec<String>,
    config: TSLanguageConfig,
    lois: HashSet<usize>,
    show_lines: HashSet<usize>, // row numbers
    num_lines: usize,
    lines: Vec<String>,
    done_parent_scopes: HashSet<usize>,
    scopes: Vec<HashSet<usize>>, // the starting lines of the nodes that span the line
    header: Vec<Vec<(usize, usize, usize)>>, // the size, start line, end line of the nodes that span the line
    nodes: Vec<Vec<Node<'a>>>,               // tree-sitter node requires lifetime parameter
}

impl<'a> TreeContext<'a> {
    pub fn new(filename: String, code: String) -> Self {
        let ts_parsing = TSLanguageParsing::init();
        let config = ts_parsing.for_file_path(&filename).unwrap().clone();
        let lines: Vec<String> = code.split('\n').map(|s| s.to_string()).collect();
        let num_lines = lines.len() + 1;

        Self {
            filename,
            code,
            parent_context: true,
            child_context: true,
            last_line: true,
            margin: 3,
            mark_lois: true,
            header_max: 10,
            show_top_of_file_parent_scope: false,
            loi_pad: 1,
            output: vec![],
            config,
            lois: HashSet::new(),
            show_lines: HashSet::new(),
            num_lines,
            lines,
            done_parent_scopes: HashSet::new(),
            scopes: vec![HashSet::new(); num_lines],
            header: vec![Vec::new(); num_lines],
            nodes: vec![Vec::new(); num_lines],
        }
    }

    pub fn get_config(&self) -> &TSLanguageConfig {
        &self.config
    }

    /// add lines of interest to the context
    pub fn add_lois(&mut self, lois: Vec<usize>) {
        self.lois.extend(lois);
    }

    pub fn add_context(&mut self) {
        if self.lois.is_empty() {
            return;
        }

        self.show_lines = self.lois.clone();

        if self.loi_pad > 0 {
            // for each interesting line
            for line in self.show_lines.clone().iter() {
                // for each of their surrounding lines
                for new_line in
                    line.saturating_sub(self.loi_pad)..=line.saturating_add(self.loi_pad)
                // since new_line usize could be negative
                {
                    if new_line >= self.num_lines {
                        continue;
                    }

                    self.show_lines.insert(new_line);
                }
            }
        }

        if self.last_line {
            // add the bottom line
            let bottom_line = self.num_lines - 2;
            self.show_lines.insert(bottom_line);
            self.add_parent_scopes(bottom_line);
        }

        todo!()
    }

    fn get_last_line_of_scope(&self, index: usize) -> usize {
        self.nodes[index]
            .iter()
            .map(|node| node.end_position().row)
            .max()
            .unwrap()
    }

    pub fn add_parent_scopes(&mut self, index: usize) {
        if self.done_parent_scopes.contains(&index) {
            return;
        }

        self.done_parent_scopes.insert(index);

        for line_num in self.scopes[index].clone().iter() {
            let (size, head_start, head_end) = self.header[*line_num].first().unwrap();

            if head_start > &0 || self.show_top_of_file_parent_scope {
                self.show_lines.extend(*head_start..*head_end);
            }

            if self.last_line {
                let last_line = self.get_last_line_of_scope(*line_num);
                self.add_parent_scopes(last_line);
            }
        }
    }

    pub fn print_tree(&self) {
        let tree = self
            .config
            .get_tree_sitter_tree(&self.code.as_bytes())
            .unwrap();

        let cursor = tree.walk();

        let mut printer = TreePrinter::new(cursor, self.code.clone()).unwrap();
        printer.walk_tree();
    }
}
