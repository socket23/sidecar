//! Contains the environment event which might be sent externally
//! to either inform of something happening or for a request-id

use crate::agentic::symbol::types::SymbolEventRequest;

use super::{input::SymbolEventRequestId, lsp::LSPSignal};

pub enum EnvironmentEventType {
    Symbol(SymbolEventRequest),
    LSP(LSPSignal),
    Human(String),
    ShutDown,
}

pub struct EnvironmentEvent {
    _request_id: SymbolEventRequestId,
    _event: EnvironmentEventType,
}

impl EnvironmentEvent {
    /// Creates a lsp signal
    pub fn lsp_signal(request_id: SymbolEventRequestId, signal: LSPSignal) -> Self {
        Self {
            _request_id: request_id,
            _event: EnvironmentEventType::LSP(signal),
        }
    }
}
