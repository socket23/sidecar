use crate::chunking::languages::{TSLanguageConfig, TSLanguageParsing};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("No language configuration found for file: {0}")]
    MissingConfig(String),
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
}

impl Default for TreeContext {
    fn default() -> Self {
        Self {
            filename: "".to_string(),
            code: "".to_string(),
            line_number: false,
            parent_context: true,
            child_context: true,
            last_line: true,
            margin: 3,
            mark_lois: true,
            header_max: 10,
            show_top_of_file_parent_scope: false,
            loi_pad: 1,
        }
    }
}

impl TreeContext {
    pub fn new(filename: String, code: String, ts_parser: &Arc<TSLanguageParsing>) -> Self {
        Self {
            filename,
            code,
            ..Default::default()
        }
    }

    // todo: get parser for language
    fn get_ts_config(
        &self,
        ts_parsing: &TSLanguageParsing,
    ) -> Result<TSLanguageConfig, ConfigError> {
        match ts_parsing.for_file_path(&self.filename) {
            Some(config) => Ok(config.clone()),
            None => Err(ConfigError::MissingConfig(self.filename.clone())),
        }
    }

    // todo: get tree from parser

    // split code into lines

    // get lines count

    // initialise output lines HashMap

    // initialise scopes, headers, nodes

    // get root node

    // walk tree

    // add lines of interest (lois)

    // add context()

    // format
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_context_default() {
        // Act
        let default_context = TreeContext::default();

        // Assert
        assert_eq!(default_context.filename, "");
        assert_eq!(default_context.code, "");
        assert_eq!(default_context.line_number, false);
        assert_eq!(default_context.parent_context, true);
        assert_eq!(default_context.child_context, true);
        assert_eq!(default_context.last_line, true);
        assert_eq!(default_context.margin, 3);
        assert_eq!(default_context.mark_lois, true);
        assert_eq!(default_context.header_max, 10);
        assert_eq!(default_context.show_top_of_file_parent_scope, false);
        assert_eq!(default_context.loi_pad, 1);
    }

    #[test]
    fn test_get_ts_config_success() {
        let ts_parsing = Arc::new(TSLanguageParsing::init());
        let context = TreeContext::new("test.ts".to_string(), "".to_string(), &ts_parsing);
        let config = context.get_ts_config(&ts_parsing).unwrap();

        assert_eq!(config.file_extensions.contains(&"ts"), true);
        assert_eq!(config.file_extensions.contains(&"tsx"), true);
    }

    #[test]
    fn test_get_ts_config_failure() {
        let ts_parsing = Arc::new(TSLanguageParsing::init());
        let context = TreeContext::new("nonexistent.xyz".to_string(), "".to_string(), &ts_parsing);
        let result = context.get_ts_config(&ts_parsing);

        assert!(result.is_err());
        match result {
            Err(ConfigError::MissingConfig(filename)) => {
                assert_eq!(filename, "nonexistent.xyz");
            }
            _ => panic!("Expected MissingConfig error"),
        }
    }
}
