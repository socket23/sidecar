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

use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::chunking::{editor_parsing::EditorParsing, text_document::Range};

use super::document::content::{DocumentEditLines, SnippetInformation};

const MAX_HISTORY_SIZE: usize = 50;
const MAX_HISTORY_SIZE_FOR_CODE_SNIPPETS: usize = 20;

/// This is the symbol tracker which will be used for inline completion
/// We keep track of the document histories and the content of these documents
pub struct SymbolTrackerInline {
    // We are storing the fs path of the documents, these are stored in the reverse
    // order
    document_history: Mutex<Vec<String>>,
    document_lines: Mutex<HashMap<String, DocumentEditLines>>,
    editor_parsing: Arc<EditorParsing>,
}

impl SymbolTrackerInline {
    pub fn new(editor_parsing: Arc<EditorParsing>) -> SymbolTrackerInline {
        SymbolTrackerInline {
            document_history: Mutex::new(Vec::new()),
            document_lines: Mutex::new(HashMap::new()),
            editor_parsing,
        }
    }

    pub async fn track_file(&self, document_path: String) {
        // First we check if the document is already present in the history
        {
            let mut document_history = self.document_history.lock().await;
            if !document_history.contains(&document_path) {
                document_history.push(document_path.to_owned());
                if document_history.len() > MAX_HISTORY_SIZE {
                    document_history.remove(0);
                }
            } else {
                let index = document_history
                    .iter()
                    .position(|x| x == &document_path)
                    .unwrap();
                document_history.remove(index);
                document_history.push(document_path.to_owned());
            }
        }
    }

    pub async fn get_document_lines(
        &self,
        file_path: &str,
        context_to_compare: &str,
    ) -> Option<Vec<SnippetInformation>> {
        {
            let document_lines = self.document_lines.lock().await;
            if let Some(document_lines_entry) = document_lines.get(file_path) {
                return Some(document_lines_entry.grab_similar_context(context_to_compare));
            }
        }
        None
    }

    pub async fn add_document(&self, document_path: String, content: String, language: String) {
        // First we check if the document is already present in the history
        self.track_file(document_path.to_owned()).await;
        // Next we will create an entry in the document lines if it does not exist
        {
            let mut document_lines = self.document_lines.lock().await;
            let document_lines_entry = DocumentEditLines::new(
                document_path.to_owned(),
                content,
                language,
                self.editor_parsing.clone(),
            );
            document_lines.insert(document_path.clone(), document_lines_entry);
        }
    }

    pub async fn file_content_change(&self, document_path: String, edits: Vec<(Range, String)>) {
        // always track the file which is being edited
        self.track_file(document_path.to_owned()).await;
        // Now we first need to get the lock over the document lines
        // and then iterate over all the edits and apply them
        let mut document_lines = self.document_lines.lock().await;
        let document_lines_entry = document_lines.get_mut(&document_path).unwrap();
        for (range, new_text) in edits {
            document_lines_entry.content_change(range, new_text);
        }
    }

    pub async fn get_document_history(&self) -> Vec<String> {
        // only get the MAX_HISTORY_SIZE_FOR_CODE_SNIPPETS size from the history
        let document_history = self.document_history.lock().await;
        document_history
            .iter()
            // we need it in the reverse order
            .rev()
            .take(MAX_HISTORY_SIZE_FOR_CODE_SNIPPETS)
            .map(|x| x.clone())
            .collect()
    }
}
