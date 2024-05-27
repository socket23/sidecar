//! We are going to send a probing request over here
//! to ask for more questions

use crate::agentic::symbol::identifier::SymbolIdentifier;

#[derive(Debug, Clone)]
pub struct SymbolToProbeHistory {
    symbol: String,
    fs_file_path: String,
    content: String,
}

#[derive(Debug, Clone)]
pub struct SymbolToProbeRequest {
    symbol_identifier: SymbolIdentifier,
    probe_request: String,
    history: Vec<SymbolToProbeHistory>,
}

impl SymbolToProbeRequest {
    pub fn new(
        symbol_identifier: SymbolIdentifier,
        probe_request: String,
        history: Vec<SymbolToProbeHistory>,
    ) -> Self {
        Self {
            symbol_identifier,
            probe_request,
            history,
        }
    }
}
