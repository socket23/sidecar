//! The message event which we send between different symbols
//! Keeps all the events which are sending intact

use crate::agentic::symbol::{
    types::{SymbolEventRequest, SymbolEventResponse},
    ui_event::UIEventWithID,
};

use super::input::SymbolEventRequestId;

/// The properties which get sent along with each symbol event
#[derive(Clone)]
pub struct SymbolEventMessageProperties {
    request_id: SymbolEventRequestId,
    ui_sender: tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
    editor_url: String,
}

impl SymbolEventMessageProperties {
    pub fn new(
        request_id: SymbolEventRequestId,
        ui_sender: tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
        editor_url: String,
    ) -> Self {
        Self {
            request_id,
            ui_sender,
            editor_url,
        }
    }

    pub fn editor_url(&self) -> String {
        self.editor_url.to_owned()
    }

    pub fn request_id_str(&self) -> &str {
        self.request_id.request_id()
    }

    pub fn root_request_id(&self) -> &str {
        self.request_id.root_request_id()
    }

    pub fn ui_sender(&self) -> tokio::sync::mpsc::UnboundedSender<UIEventWithID> {
        self.ui_sender.clone()
    }

    pub fn request_id(&self) -> &SymbolEventRequestId {
        &self.request_id
    }

    pub fn set_request_id(mut self, request_id: String) -> Self {
        self.request_id = self.request_id.set_request_id(request_id);
        self
    }
}

/// The properties which get sent along with a symbol request across
/// the channels
///
/// This also carries the metadata and request_id as well
pub struct SymbolEventMessage {
    symbol_event_request: SymbolEventRequest,
    response_sender: tokio::sync::oneshot::Sender<SymbolEventResponse>,
    properties: SymbolEventMessageProperties,
}

impl SymbolEventMessage {
    pub fn new(
        symbol_event_request: SymbolEventRequest,
        request_id: SymbolEventRequestId,
        ui_sender: tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
        response_sender: tokio::sync::oneshot::Sender<SymbolEventResponse>,
        editor_url: String,
    ) -> Self {
        Self {
            symbol_event_request,
            properties: SymbolEventMessageProperties::new(request_id, ui_sender, editor_url),
            response_sender,
        }
    }

    pub fn get_properties(&self) -> &SymbolEventMessageProperties {
        &self.properties
    }

    pub fn message_with_properties(
        symbol_event_request: SymbolEventRequest,
        properties: SymbolEventMessageProperties,
        response_sender: tokio::sync::oneshot::Sender<SymbolEventResponse>,
    ) -> Self {
        Self {
            symbol_event_request,
            properties,
            response_sender,
        }
    }

    pub fn symbol_event_request(&self) -> &SymbolEventRequest {
        &self.symbol_event_request
    }

    pub fn request_id_data(&self) -> SymbolEventRequestId {
        self.properties.request_id.clone()
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

    pub fn remove_response_sender(self) -> tokio::sync::oneshot::Sender<SymbolEventResponse> {
        self.response_sender
    }
}
