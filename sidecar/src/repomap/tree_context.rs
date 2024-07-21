use crate::chunking::languages::{TSLanguageConfig, TSLanguageParsing};

use super::tree_printer::TreePrinter;

pub struct TreeContext {
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
}

impl TreeContext {
    pub fn new(filename: String, code: String) -> Self {
        let ts_parsing = TSLanguageParsing::init();
        let config = ts_parsing.for_file_path(&filename).unwrap().clone();
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
        }
    }

    pub fn get_config(&self) -> &TSLanguageConfig {
        &self.config
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
