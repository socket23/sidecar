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

use crate::chunking::{
    editor_parsing::EditorParsing,
    text_document::{Position, Range},
};

use super::document::content::{DocumentEditLines, SnippetInformationWithScore};

const MAX_HISTORY_SIZE: usize = 50;
const MAX_HISTORY_SIZE_FOR_CODE_SNIPPETS: usize = 20;

struct GetDocumentLinesRequest {
    file_path: String,
    context_to_compare: String,
    skip_line: Option<usize>,
}

impl GetDocumentLinesRequest {
    pub fn new(file_path: String, context_to_compare: String, skip_line: Option<usize>) -> Self {
        Self {
            file_path,
            context_to_compare,
            skip_line,
        }
    }
}

struct AddDocumentRequest {
    document_path: String,
    language: String,
    content: String,
}

impl AddDocumentRequest {
    pub fn new(document_path: String, language: String, content: String) -> Self {
        Self {
            document_path,
            language,
            content,
        }
    }
}

struct FileContentChangeRequest {
    document_path: String,
    file_content: String,
    language: String,
    edits: Vec<(Range, String)>,
}

impl FileContentChangeRequest {
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

struct GetFileEditedLinesRequest {
    file_path: String,
}

impl GetFileEditedLinesRequest {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }
}

struct GetIdentifierNodesRequest {
    file_path: String,
    cursor_position: Position,
}

enum SharedStateRequest {
    GetFileContent(String),
    GetDocumentLines(GetDocumentLinesRequest),
    AddDocument(AddDocumentRequest),
    FileContentChange(FileContentChangeRequest),
    GetDocumentHistory,
    GetFileEditedLines(GetFileEditedLinesRequest),
    GetIdentifierNodes(GetIdentifierNodesRequest),
}

enum SharedStateResponse {
    DocumentHistoryResponse(Vec<String>),
    Ok,
    FileContentRespone(Option<String>),
    GetDocumentLinesResponse(Option<Vec<SnippetInformationWithScore>>),
    FileEditedLinesResponse(Vec<usize>),
    GetIdentifierNodesResponse(Vec<(String, Range)>),
}

pub struct SharedState {
    document_lines: Mutex<HashMap<String, DocumentEditLines>>,
    document_history: Mutex<Vec<String>>,
    editor_parsing: Arc<EditorParsing>,
}

impl SharedState {
    async fn process_request(&self, request: SharedStateRequest) -> SharedStateResponse {
        match request {
            SharedStateRequest::AddDocument(add_document_request) => {
                let _ = self
                    .add_document(
                        add_document_request.document_path,
                        add_document_request.content,
                        add_document_request.language,
                    )
                    .await;
                SharedStateResponse::Ok
            }
            SharedStateRequest::FileContentChange(file_content_change_request) => {
                let _ = self
                    .file_content_change(
                        file_content_change_request.document_path,
                        file_content_change_request.file_content,
                        file_content_change_request.language,
                        file_content_change_request.edits,
                    )
                    .await;
                SharedStateResponse::Ok
            }
            SharedStateRequest::GetDocumentLines(get_document_lines_request) => {
                let response = self
                    .get_document_lines(
                        &get_document_lines_request.file_path,
                        &get_document_lines_request.context_to_compare,
                        get_document_lines_request.skip_line,
                    )
                    .await;
                SharedStateResponse::GetDocumentLinesResponse(response)
            }
            SharedStateRequest::GetFileContent(get_file_content_request) => {
                let response = self.get_file_content(&get_file_content_request).await;
                SharedStateResponse::FileContentRespone(response)
            }
            SharedStateRequest::GetDocumentHistory => {
                let response = self.get_document_history().await;
                SharedStateResponse::DocumentHistoryResponse(response)
            }
            SharedStateRequest::GetFileEditedLines(file_request) => {
                let response = self.get_edited_lines(&file_request.file_path).await;
                SharedStateResponse::FileEditedLinesResponse(response)
            }
            SharedStateRequest::GetIdentifierNodes(request) => {
                let file_path = request.file_path;
                let position = request.cursor_position;
                let response = self.get_identifier_nodes(&file_path, position).await;
                SharedStateResponse::GetIdentifierNodesResponse(response)
            }
        }
    }

    async fn get_file_content(&self, file_path: &str) -> Option<String> {
        let document_lines = self.document_lines.lock().await;
        document_lines
            .get(file_path)
            .map(|document_lines| document_lines.get_content())
    }

    async fn track_file(&self, document_path: String) {
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

    async fn get_edited_lines(&self, file_path: &str) -> Vec<usize> {
        {
            let mut document_lines = self.document_lines.lock().await;
            if let Some(ref mut document_lines_entry) = document_lines.get_mut(file_path) {
                return document_lines_entry.get_edited_lines();
            }
        }
        Vec::new()
    }

    async fn get_identifier_nodes(
        &self,
        file_path: &str,
        position: Position,
    ) -> Vec<(String, Range)> {
        {
            let mut document_lines = self.document_lines.lock().await;
            if let Some(ref mut document_lines_entry) = document_lines.get_mut(file_path) {
                return document_lines_entry.get_identifier_nodes(position);
            }
        }
        Default::default()
    }

    async fn get_document_lines(
        &self,
        file_path: &str,
        context_to_compare: &str,
        skip_line: Option<usize>,
    ) -> Option<Vec<SnippetInformationWithScore>> {
        {
            let mut document_lines = self.document_lines.lock().await;
            if let Some(ref mut document_lines_entry) = document_lines.get_mut(file_path) {
                return Some(
                    document_lines_entry.grab_similar_context(context_to_compare, skip_line),
                );
            }
        }
        None
    }

    async fn add_document(&self, document_path: String, content: String, language: String) {
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

    async fn file_content_change(
        &self,
        document_path: String,
        file_content: String,
        language: String,
        edits: Vec<(Range, String)>,
    ) {
        // always track the file which is being edited
        self.track_file(document_path.to_owned()).await;
        if edits.is_empty() {
            return;
        }
        // Now we first need to get the lock over the document lines
        // and then iterate over all the edits and apply them
        let mut document_lines = self.document_lines.lock().await;

        // If we do not have the document (which can happen if the sidecar restarts, just add it
        // and do not do anything about the edits yet)
        if !document_lines.contains_key(&document_path) {
            let document_lines_entry = DocumentEditLines::new(
                document_path.to_owned(),
                file_content,
                language,
                self.editor_parsing.clone(),
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
                        self.editor_parsing.clone(),
                    );
                    document_lines.insert(document_path.clone(), document_lines_entry);
                }
            }
        }
    }

    async fn get_document_history(&self) -> Vec<String> {
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
    sender: tokio::sync::mpsc::UnboundedSender<(
        SharedStateRequest,
        tokio::sync::oneshot::Sender<SharedStateResponse>,
    )>,
}

impl SymbolTrackerInline {
    pub fn new(editor_parsing: Arc<EditorParsing>) -> SymbolTrackerInline {
        let shared_state = Arc::new(SharedState {
            document_lines: Mutex::new(HashMap::new()),
            document_history: Mutex::new(Vec::new()),
            editor_parsing: editor_parsing.clone(),
        });
        let shared_state_cloned = shared_state.clone();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<(
            SharedStateRequest,
            tokio::sync::oneshot::Sender<SharedStateResponse>,
        )>();

        // start a background thread with the receiver
        tokio::spawn(async move {
            let shared_state = shared_state_cloned.clone();
            while let Some(value) = receiver.recv().await {
                let request = value.0;
                let sender = value.1;
                let response = shared_state.process_request(request).await;
                let _ = sender.send(response);
            }
        });
        SymbolTrackerInline {
            symbol_tracker_state: shared_state.clone(),
            sender,
        }
    }

    pub async fn get_file_content(&self, file_path: &str) -> Option<String> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request = SharedStateRequest::GetFileContent(file_path.to_owned());
        let _ = self.sender.send((request, sender));
        let reply = receiver.await;
        if let Ok(SharedStateResponse::FileContentRespone(response)) = reply {
            response
        } else {
            None
        }
    }

    pub async fn get_file_edited_lines(&self, file_path: &str) -> Vec<usize> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request = SharedStateRequest::GetFileEditedLines(GetFileEditedLinesRequest::new(
            file_path.to_owned(),
        ));
        let _ = self.sender.send((request, sender));
        let reply = receiver.await;
        if let Ok(SharedStateResponse::FileEditedLinesResponse(response)) = reply {
            response
        } else {
            Vec::new()
        }
    }

    //
    pub async fn get_document_lines(
        &self,
        file_path: &str,
        context_to_compare: &str,
        skip_line: Option<usize>,
    ) -> Option<Vec<SnippetInformationWithScore>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request = SharedStateRequest::GetDocumentLines(GetDocumentLinesRequest::new(
            file_path.to_owned(),
            context_to_compare.to_owned(),
            skip_line,
        ));
        let _ = self.sender.send((request, sender));
        let reply = receiver.await;
        if let Ok(SharedStateResponse::GetDocumentLinesResponse(response)) = reply {
            response
        } else {
            None
        }
    }

    pub async fn add_document(&self, document_path: String, content: String, language: String) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request = SharedStateRequest::AddDocument(AddDocumentRequest::new(
            document_path,
            language,
            content,
        ));
        let _ = self.sender.send((request, sender));
        let _ = receiver.await;
    }

    pub async fn file_content_change(
        &self,
        document_path: String,
        file_content: String,
        language: String,
        edits: Vec<(Range, String)>,
    ) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request = SharedStateRequest::FileContentChange(FileContentChangeRequest::new(
            document_path,
            file_content,
            language,
            edits,
        ));
        let _ = self.sender.send((request, sender));
        let _ = receiver.await;
    }

    pub async fn get_document_history(&self) -> Vec<String> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request = SharedStateRequest::GetDocumentHistory;
        let _ = self.sender.send((request, sender));
        let reply = receiver.await;
        if let Ok(SharedStateResponse::DocumentHistoryResponse(response)) = reply {
            response
        } else {
            vec![]
        }
    }

    pub async fn get_identifier_nodes(
        &self,
        file_path: &str,
        position: Position,
    ) -> Vec<(String, Range)> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request = SharedStateRequest::GetIdentifierNodes(GetIdentifierNodesRequest {
            file_path: file_path.to_owned(),
            cursor_position: position,
        });
        let _ = self.sender.send((request, sender));
        let reply = receiver.await;
        if let Ok(SharedStateResponse::GetIdentifierNodesResponse(response)) = reply {
            response
        } else {
            Default::default()
        }
    }
}
