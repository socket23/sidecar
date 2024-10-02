//! Contains the LSP signal which might be sent from the editor
//! For now, its just the diagnostics when we detect a change in the editor

use crate::chunking::text_document::Range;

#[derive(Debug, Clone)]
pub struct LSPDiagnosticError {
    range: Range,
    snippet: String,
    fs_file_path: String,
    diagnostic: String,
    associated_files: Option<Vec<String>>,
}

impl LSPDiagnosticError {
    pub fn new(range: Range, snippet: String, fs_file_path: String, diagnostic: String) -> Self {
        Self {
            range,
            snippet,
            fs_file_path,
            diagnostic,
            associated_files: None,
        }
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn diagnostic_message(&self) -> &str {
        &self.diagnostic
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn snippet(&self) -> &str {
        &self.snippet
    }

    pub fn associated_files(&self) -> Option<&Vec<String>> {
        self.associated_files.as_ref()
    }

    pub fn set_associated_files(&mut self, files: Vec<String>) {
        self.associated_files = Some(files);
    }
}

/// Contains the different lsp signals which we get from the editor
/// instead of being poll based we can get a push event over here
pub enum LSPSignal {
    Diagnostics(Vec<LSPDiagnosticError>),
}

impl LSPSignal {
    pub fn diagnostics(diagnostics: Vec<LSPDiagnosticError>) -> Self {
        LSPSignal::Diagnostics(diagnostics)
    }
}
