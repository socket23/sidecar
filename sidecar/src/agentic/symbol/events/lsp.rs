//! Contains the LSP signal which might be sent from the editor
//! For now, its just the diagnostics when we detect a change in the editor

use crate::chunking::text_document::Range;

pub struct LSPDiagnosticError {
    _range: Range,
    _fs_file_path: String,
    _diagnostic: String,
}

/// Contains the different lsp signals which we get from the editor
/// instead of being poll based we can get a push event over here
pub enum LSPSignal {
    Diagnostics(LSPDiagnosticError),
}
