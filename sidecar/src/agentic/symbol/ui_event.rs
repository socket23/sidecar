//! We are going to log the UI events, this is mostly for
//! debugging and having better visibility to what ever is happening
//! in the symbols

use crate::agentic::tool::input::ToolInput;

use super::{events::input::SymbolInputEvent, types::SymbolEventRequest};

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
}

#[derive(Debug, serde::Serialize)]
pub enum UIEvent {
    SymbolEvent(SymbolEventRequest),
    ToolEvent(ToolInput),
    CodebaseEvent(SymbolInputEvent),
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
