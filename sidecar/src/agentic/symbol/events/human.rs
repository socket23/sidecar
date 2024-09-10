//! Contains the different kind of messages which are coming from the human

use crate::agentic::symbol::anchored::AnchoredSymbol;

#[derive(Debug)]
pub struct HumanAnchorRequest {
    query: String,
    anchored_symbols: Vec<AnchoredSymbol>,
    anchor_request_context: Option<String>,
}

impl HumanAnchorRequest {
    pub fn new(
        query: String,
        anchored_symbols: Vec<AnchoredSymbol>,
        anchor_request_context: Option<String>,
    ) -> Self {
        Self {
            query,
            anchored_symbols,
            anchor_request_context,
        }
    }

    pub fn anchored_symbols(&self) -> &[AnchoredSymbol] {
        self.anchored_symbols.as_slice()
    }

    pub fn user_query(&self) -> &str {
        &self.query
    }

    pub fn anchor_request_context(&self) -> Option<String> {
        self.anchor_request_context.clone()
    }
}

#[derive(Debug)]
pub enum HumanMessage {
    Followup(String),
    Anchor(HumanAnchorRequest),
}
