//! We are going to log the UI events, this is mostly for
//! debugging and having better visibility to what ever is happening
//! in the symbols

use crate::agentic::tool::{filtering::broker::CodeToProbeFilterResponse, input::ToolInput};

use super::{
    events::input::SymbolInputEvent,
    identifier::SymbolIdentifier,
    types::{SymbolEventRequest, SymbolLocation},
};

#[derive(Debug, serde::Serialize)]
pub struct UIEventWithID {
    request_id: String,
    event: UIEvent,
}

impl UIEventWithID {
    pub fn from_tool_event(request_id: String, input: ToolInput) -> Self {
        Self {
            request_id,
            event: UIEvent::from(input),
        }
    }

    pub fn from_symbol_event(request_id: String, input: SymbolEventRequest) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolEvent(input),
        }
    }

    pub fn for_codebase_event(request_id: String, input: SymbolInputEvent) -> Self {
        Self {
            request_id,
            event: UIEvent::CodebaseEvent(input),
        }
    }

    pub fn symbol_location(request_id: String, symbol_location: SymbolLocation) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolLoctationUpdate(symbol_location),
        }
    }

    pub fn sub_symbol_step(
        request_id: String,
        sub_symbol_request: SymbolEventSubStepRequest,
    ) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolEventSubStep(sub_symbol_request),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub enum UIEvent {
    SymbolEvent(SymbolEventRequest),
    ToolEvent(ToolInput),
    CodebaseEvent(SymbolInputEvent),
    SymbolLoctationUpdate(SymbolLocation),
    SymbolEventSubStep(SymbolEventSubStepRequest),
}

impl From<SymbolEventRequest> for UIEvent {
    fn from(req: SymbolEventRequest) -> Self {
        UIEvent::SymbolEvent(req)
    }
}

impl From<ToolInput> for UIEvent {
    fn from(input: ToolInput) -> Self {
        UIEvent::ToolEvent(input)
    }
}

#[derive(Debug, serde::Serialize)]
pub enum SymbolEventProbeRequest {
    SubSymbolSelection,
    ProbeDeeperSymbol,
}

#[derive(Debug, serde::Serialize)]
pub enum SymbolEventSubStep {
    Probe(SymbolEventProbeRequest),
}

#[derive(Debug, serde::Serialize)]
pub struct SymbolEventSubStepRequest {
    symbol_identifier: SymbolIdentifier,
    event: SymbolEventSubStep,
}

impl SymbolEventSubStepRequest {
    pub fn new(symbol_identifier: SymbolIdentifier, event: SymbolEventSubStep) -> Self {
        Self {
            symbol_identifier,
            event,
        }
    }
}
