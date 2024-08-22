use crate::{agentic::symbol::identifier::SymbolIdentifier, chunking::text_document::Range};

use super::initial_request::{SymbolEditedItem, SymbolRequestHistoryItem};

#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolToEdit {
    outline: bool, // todo(zi): remove this mfer, test case
    range: Range,
    fs_file_path: String,
    symbol_name: String,
    instructions: Vec<String>,
    is_new: bool,
    // If this is a full symbol edit instead of being sub-symbol level
    is_full_edit: bool, // todo(zi): remove this mfer, test case 2
    original_user_query: String,
    symbol_edited_list: Option<Vec<SymbolEditedItem>>,
    // If we should gather definitions for editing
    gather_definitions_for_editing: bool,
    // user provided context as a string for the LLM to use
    user_provided_context: Option<String>,
    // Whether to disable followups and correctness checks
    disable_followups_and_correctness: bool,
    // if we should apply the edits directly
    apply_edits_directly: bool,
}

impl SymbolToEdit {
    pub fn new(
        symbol_name: String,
        range: Range,
        fs_file_path: String,
        instructions: Vec<String>,
        outline: bool,
        is_new: bool,
        is_full_edit: bool,
        original_user_query: String,
        symbol_edited_list: Option<Vec<SymbolEditedItem>>,
        gather_definitions_for_editing: bool,
        user_provided_context: Option<String>,
        disable_followups_and_correctness: bool,
    ) -> Self {
        Self {
            symbol_name,
            range,
            outline,
            fs_file_path,
            instructions,
            is_new,
            is_full_edit,
            original_user_query,
            symbol_edited_list,
            gather_definitions_for_editing,
            user_provided_context,
            disable_followups_and_correctness,
            apply_edits_directly: false,
        }
    }

    pub fn apply_edits_directly(mut self) -> Self {
        self.apply_edits_directly = true;
        self
    }

    pub fn should_apply_edits_directory(&self) -> bool {
        self.apply_edits_directly
    }

    pub fn should_disable_followups_and_correctness(&self) -> bool {
        self.disable_followups_and_correctness
    }

    pub fn should_gather_definitions_for_editing(&self) -> bool {
        self.gather_definitions_for_editing
    }

    pub fn symbol_edited_list(&self) -> Option<Vec<SymbolEditedItem>> {
        self.symbol_edited_list.clone()
    }

    pub fn original_user_query(&self) -> &str {
        &self.original_user_query
    }

    pub fn is_full_edit(&self) -> bool {
        self.is_full_edit
    }

    pub fn set_fs_file_path(&mut self, fs_file_path: String) {
        self.fs_file_path = fs_file_path;
    }

    pub fn set_range(&mut self, range: Range) {
        self.range = range;
    }

    pub fn is_new(&self) -> bool {
        self.is_new.clone()
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn is_outline(&self) -> bool {
        self.outline
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub fn instructions(&self) -> &[String] {
        self.instructions.as_slice()
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn user_provided_context(&self) -> Option<String> {
        self.user_provided_context.clone()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolToEditRequest {
    symbols: Vec<SymbolToEdit>,
    symbol_identifier: SymbolIdentifier,
    history: Vec<SymbolRequestHistoryItem>,
}

impl SymbolToEditRequest {
    pub fn new(
        symbols: Vec<SymbolToEdit>,
        identifier: SymbolIdentifier,
        history: Vec<SymbolRequestHistoryItem>,
    ) -> Self {
        Self {
            symbol_identifier: identifier,
            symbols,
            history,
        }
    }

    pub fn symbols(self) -> Vec<SymbolToEdit> {
        self.symbols
    }

    pub fn symbol_identifier(&self) -> &SymbolIdentifier {
        &self.symbol_identifier
    }

    pub fn history(&self) -> &[SymbolRequestHistoryItem] {
        self.history.as_slice()
    }
}
