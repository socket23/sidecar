//! The different kind of events which the symbols can invoke and needs to work
//! on

use super::edit::SymbolToEditRequest;

#[derive(Debug, Clone)]
pub enum SymbolEvent {
    InitialRequest,
    AskQuestion,
    UserFeedback,
    Delete,
    Edit(SymbolToEditRequest),
}
