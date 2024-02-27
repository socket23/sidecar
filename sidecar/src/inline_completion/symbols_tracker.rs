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

use tokio::sync::Mutex;

const MAX_HISTORY_SIZE: usize = 50;
const MAX_HISTORY_SIZE_FOR_CODE_SNIPPETS: usize = 20;

/// This is the symbol tracker which will be used for inline completion
/// We keep track of the document histories and the content of these documents
pub struct SymbolTrackerInline {
    // We are storing the fs path of the documents, these are stored in the reverse
    // order
    pub document_history: Mutex<Vec<String>>,
}

impl SymbolTrackerInline {
    pub fn new() -> SymbolTrackerInline {
        SymbolTrackerInline {
            document_history: Mutex::new(Vec::new()),
        }
    }

    pub async fn add_document(&self, document_path: String) {
        // First we check if the document is already present in the history
        let mut document_history = self.document_history.lock().await;
        if !document_history.contains(&document_path) {
            document_history.push(document_path);
            if document_history.len() > MAX_HISTORY_SIZE {
                document_history.remove(0);
            }
        } else {
            // We are going to move the document to the end of the history
            let index = document_history
                .iter()
                .position(|x| x == &document_path)
                .unwrap();
            document_history.remove(index);
            document_history.push(document_path);
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
