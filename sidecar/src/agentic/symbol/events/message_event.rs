//! The message event which we send between different symbols
//! Keeps all the events which are sending intact

use crate::agentic::symbol::{
    types::{SymbolEventRequest, SymbolEventResponse},
    ui_event::UIEventWithID,
};

use super::input::SymbolEventRequestId;

/// The properties which get sent along with each symbol event
pub struct SymbolEventMessageProperties {
    request_id: SymbolEventRequestId,
    ui_sender: tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
    response_sender: tokio::sync::oneshot::Sender<SymbolEventResponse>,
}

impl SymbolEventMessageProperties {
    pub fn new(
        request_id: SymbolEventRequestId,
        ui_sender: tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
        response_sender: tokio::sync::oneshot::Sender<SymbolEventResponse>,
    ) -> Self {
        Self {
            request_id,
            ui_sender,
            response_sender,
        }
    }
}

/// The properties which get sent along with a symbol request across
/// the channels
///
/// This also carries the metadata and request_id as well
pub struct SymbolEventMessage {
    symbol_event_request: SymbolEventRequest,
    properties: SymbolEventMessageProperties,
}

impl SymbolEventMessage {
    pub fn new(
        symbol_event_request: SymbolEventRequest,
        request_id: SymbolEventRequestId,
        ui_sender: tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
        response_sender: tokio::sync::oneshot::Sender<SymbolEventResponse>,
    ) -> Self {
        Self {
            symbol_event_request,
            properties: SymbolEventMessageProperties::new(request_id, ui_sender, response_sender),
        }
    }

    pub fn symbol_event_request(&self) -> &SymbolEventRequest {
        &self.symbol_event_request
    }

    pub fn request_id(&self) -> &str {
        self.properties.request_id.request_id()
    }

    pub fn root_request_id(&self) -> &str {
        self.properties.request_id.root_request_id()
    }

    pub fn ui_sender(&self) -> tokio::sync::mpsc::UnboundedSender<UIEventWithID> {
        self.properties.ui_sender.clone()
    }

    pub fn response_sender(&self) -> &tokio::sync::oneshot::Sender<SymbolEventResponse> {
        &self.properties.response_sender
    }
}
