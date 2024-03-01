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

use super::document::content::{DocumentEditLines, SnippetInformationWithScore};

const MAX_HISTORY_SIZE: usize = 50;
const MAX_HISTORY_SIZE_FOR_CODE_SNIPPETS: usize = 20;

pub struct SharedState {
    document_lines: Mutex<HashMap<String, DocumentEditLines>>,
    document_history: Mutex<Vec<String>>,
    editor_parsing: Arc<EditorParsing>,
}

pub struct EditRequest {
    document_path: String,
    file_content: String,
    language: String,
    edits: Vec<(Range, String)>,
}

impl EditRequest {
    pub fn new(
        document_path: String,
        file_content: String,
        language: String,
        edits: Vec<(Range, String)>,
    ) -> Self {
        Self {
            document_path,
            file_content,
            language,
            edits,
        }
    }
}

/// This is the symbol tracker which will be used for inline completion
/// We keep track of the document histories and the content of these documents
pub struct SymbolTrackerInline {
    // We are storing the fs path of the documents, these are stored in the reverse
    // order
    symbol_tracker_state: Arc<SharedState>,
    sender: tokio::sync::mpsc::UnboundedSender<EditRequest>,
}

impl SymbolTrackerInline {
    pub fn new(editor_parsing: Arc<EditorParsing>) -> SymbolTrackerInline {
        let shared_state = Arc::new(SharedState {
            document_lines: Mutex::new(HashMap::new()),
            document_history: Mutex::new(Vec::new()),
            editor_parsing: editor_parsing.clone(),
        });
        let shared_state_cloned = shared_state.clone();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<EditRequest>();

        // start a background thread with the receiver
        tokio::spawn(async move {
            let something = shared_state_cloned.clone();
            while let Some(value) = receiver.recv().await {
                let timestamp = chrono::Local::now();
                dbg!(
                    "SPANWED VALUE::::::",
                    &value.document_path,
                    &value.edits,
                    &timestamp
                );
            }
        });
        SymbolTrackerInline {
            symbol_tracker_state: shared_state.clone(),
            sender,
        }
    }

    pub async fn get_file_content(&self, file_path: &str) -> Option<String> {
        let document_lines = self.symbol_tracker_state.document_lines.lock().await;
        document_lines
            .get(file_path)
            .map(|document_lines| document_lines.get_content())
    }

    pub async fn track_file(&self, document_path: String) {
        // First we check if the document is already present in the history
        {
            let mut document_history = self.symbol_tracker_state.document_history.lock().await;
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
        skip_line: Option<usize>,
    ) -> Option<Vec<SnippetInformationWithScore>> {
        {
            let mut document_lines = self.symbol_tracker_state.document_lines.lock().await;
            if let Some(ref mut document_lines_entry) = document_lines.get_mut(file_path) {
                return Some(
                    document_lines_entry.grab_similar_context(context_to_compare, skip_line),
                );
            }
        }
        None
    }

    pub async fn add_document(&self, document_path: String, content: String, language: String) {
        // First we check if the document is already present in the history
        self.track_file(document_path.to_owned()).await;
        // Next we will create an entry in the document lines if it does not exist
        {
            let mut document_lines = self.symbol_tracker_state.document_lines.lock().await;
            let document_lines_entry = DocumentEditLines::new(
                document_path.to_owned(),
                content,
                language,
                self.symbol_tracker_state.editor_parsing.clone(),
            );
            document_lines.insert(document_path.clone(), document_lines_entry);
        }
    }

    pub async fn file_content_change(
        &self,
        document_path: String,
        file_content: String,
        language: String,
        edits: Vec<(Range, String)>,
    ) {
        let time = chrono::Local::now();
        dbg!("file_content_change", &time, &document_path, &edits);
        let edit_request = EditRequest::new(
            document_path.clone(),
            file_content.clone(),
            language.clone(),
            edits.to_vec(),
        );
        let _ = self.sender.send(edit_request);
        // always track the file which is being edited
        self.track_file(document_path.to_owned()).await;
        if edits.is_empty() {
            return;
        }
        // Now we first need to get the lock over the document lines
        // and then iterate over all the edits and apply them
        let mut document_lines = self.symbol_tracker_state.document_lines.lock().await;

        // If we do not have the document (which can happen if the sidecar restarts, just add it
        // and do not do anything about the edits yet)
        if !document_lines.contains_key(&document_path) {
            let document_lines_entry = DocumentEditLines::new(
                document_path.to_owned(),
                file_content,
                language,
                self.symbol_tracker_state.editor_parsing.clone(),
            );
            document_lines.insert(document_path.clone(), document_lines_entry);
        } else {
            let document_lines_entry = document_lines.get_mut(&document_path);
            // This match should not be required but for some reason we are hitting
            // the none case even in this branch after our checks
            match document_lines_entry {
                Some(document_lines_entry) => {
                    for (range, new_text) in edits {
                        document_lines_entry.content_change(range, new_text);
                    }
                }
                None => {
                    let document_lines_entry = DocumentEditLines::new(
                        document_path.to_owned(),
                        file_content,
                        language,
                        self.symbol_tracker_state.editor_parsing.clone(),
                    );
                    document_lines.insert(document_path.clone(), document_lines_entry);
                }
            }
        }
    }

    pub async fn get_document_history(&self) -> Vec<String> {
        // only get the MAX_HISTORY_SIZE_FOR_CODE_SNIPPETS size from the history
        let document_history = self.symbol_tracker_state.document_history.lock().await;
        document_history
            .iter()
            // we need it in the reverse order
            .rev()
            .take(MAX_HISTORY_SIZE_FOR_CODE_SNIPPETS)
            .map(|x| x.clone())
            .collect()
    }
}
