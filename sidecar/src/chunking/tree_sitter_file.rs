use std::sync::Arc;
use tree_sitter::{Parser, Tree};

use super::{
    languages::{TSLanguageConfig, TSLanguageParsing},
    scope_graph::{scope_res_generic, ScopeGraph},
    text_document::Range,
};

/// A tree-sitter representation of a file
pub struct TreeSitterFile<'a> {
    /// The original source that was used to generate this file.
    src: &'a [u8],

    /// The syntax tree of this file.
    tree: Tree,

    /// The supplied language for this file.
    language: TSLanguageConfig,
}

#[derive(Debug)]
pub enum TreeSitterFileError {
    UnsupportedLanguage,
    ParseTimeout,
    LanguageMismatch,
    QueryError(tree_sitter::QueryError),
    FileTooLarge,
}

impl<'a> TreeSitterFile<'a> {
    /// Create a TreeSitterFile out of a sourcefile
    pub fn try_build(
        src: &'a [u8],
        lang_id: &str,
        language_parsing: Arc<TSLanguageParsing>,
    ) -> Result<Self, TreeSitterFileError> {
        // no scope-res for files larger than 500kb
        if src.len() > 500 * 10usize.pow(3) {
            return Err(TreeSitterFileError::FileTooLarge);
        }

        let language_config = language_parsing.for_lang(lang_id);

        let language = match language_config {
            Some(language) => Ok(language),
            None => Err(TreeSitterFileError::UnsupportedLanguage),
        }?;

        let mut parser = Parser::new();
        parser
            .set_language((language.grammar)())
            .map_err(|_| TreeSitterFileError::LanguageMismatch)?;

        // do not permit files that take >1s to parse
        parser.set_timeout_micros(10u64.pow(6));

        let tree = parser
            .parse(src, None)
            .ok_or(TreeSitterFileError::ParseTimeout)?;

        Ok(Self {
            src,
            tree,
            language: language.clone(),
        })
    }

    /// These are all the ranges which can be hovered over in a document
    /// this helps us figure out which ranges we can perform go-to-def/reference
    /// and use that for getting more information
    pub fn hoverable_ranges(self) -> Result<Vec<Range>, TreeSitterFileError> {
        let hoverable_query = self.language.hoverable_query.to_owned();
        let query = tree_sitter::Query::new((self.language.grammar)(), &hoverable_query)
            .map_err(TreeSitterFileError::QueryError)?;
        let root_node = self.tree.root_node();
        let mut cursor = tree_sitter::QueryCursor::new();
        Ok(cursor
            .matches(&query, root_node, self.src)
            .flat_map(|m| m.captures)
            .map(|c| Range::for_tree_node(&c.node))
            .collect::<Vec<_>>())
    }

    /// Produce a lexical scope-graph for this TreeSitterFile.
    pub fn scope_graph(self) -> Result<ScopeGraph, TreeSitterFileError> {
        let scope_query = self.language.scope_query.to_owned();
        let query = tree_sitter::Query::new((self.language.grammar)(), &scope_query)
            .map_err(TreeSitterFileError::QueryError)?;
        let root_node = self.tree.root_node();

        Ok(scope_res_generic(
            &query,
            root_node,
            self.src,
            &self.language,
        ))
    }
}
