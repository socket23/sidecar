//! We are going to log the UI events, this is mostly for
//! debugging and having better visibility to what ever is happening
//! in the symbols

use crate::agentic::tool::input::ToolInput;

use super::types::SymbolEventRequest;

#[derive(Debug, serde::Serialize)]
pub enum UIEvent {
    // TODO(skcd): We need the location of the symbol over here
    SymbolEvent(SymbolEventRequest),
    // TODO(skcd): Add attribution to the symbol
    ToolEvent(ToolInput),
    // TODO(skcd): We need to send more infromation about
    // the sub-steps which the agent is taking
    SymbolEventSubStep,
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
