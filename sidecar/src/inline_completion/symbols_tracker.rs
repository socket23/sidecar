//! We are going to track the symbols here which can be because of any of the following
//! reasons:
//! - file was open in the editor
//! - file is being imported
//! - this file is part of the implementation being done in the current file (think implementations
//! of the same type)
//! - We also want to get the code snippets which have been recently edited
//! Note: this will build towards the next edit prediciton which we want to do eventually
//! Steps being taken:
//! - First we start with just the open tabs and also edit tracking here

use std::collections::HashMap;

use crate::chunking::text_document::Range;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentMetadata {
    pub file_path: String,
}

pub struct ActiveEdits {
    pub name: String,
    pub range: Range,
    pub value: String,
}

pub struct SymbolTrackerInline {
    pub active_edits: HashMap<DocumentMetadata, Vec<ActiveEdits>>,
}

impl SymbolTrackerInline {
    pub fn new() -> SymbolTrackerInline {
        SymbolTrackerInline {
            active_edits: Default::default(),
        }
    }

    pub fn add_edit(&mut self, document_metadata: DocumentMetadata, active_edit: ActiveEdits) {
        self.active_edits
            .entry(document_metadata)
            .or_insert_with(Vec::new)
            .push(active_edit);
    }
}
