//! Contains the environment event which might be sent externally
//! to either inform of something happening or for a request-id

use crate::agentic::symbol::{anchored::AnchoredSymbol, types::SymbolEventRequest};

use super::{
    human::{HumanAnchorRequest, HumanMessage},
    input::SymbolEventRequestId,
    lsp::LSPSignal,
};

pub enum EnvironmentEventType {
    Symbol(SymbolEventRequest),
    LSP(LSPSignal),
    Human(HumanMessage),
    ShutDown,
}

impl EnvironmentEventType {
    pub fn human_anchor_request(
        query: String,
        anchored_symbols: Vec<AnchoredSymbol>,
        context: Option<String>,
    ) -> Self {
        EnvironmentEventType::Human(HumanMessage::Anchor(HumanAnchorRequest::new(
            query,
            anchored_symbols,
            context,
        )))
    }
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
