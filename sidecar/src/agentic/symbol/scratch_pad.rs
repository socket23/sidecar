//! Contains the scratch pad agent whose job is to work alongside the developer
//! and help them accomplish a task
//! This way the agent can look at all the events and the requests which are happening
//! and take a decision based on them on what should happen next

use std::pin::Pin;

use futures::{Stream, StreamExt};
use tokio::sync::mpsc::UnboundedSender;

use super::events::{
    environment_event::EnvironmentEventType,
    message_event::{SymbolEventMessage, SymbolEventMessageProperties},
};

/// Different kind of events which can happen
/// We should move beyond symbol events tbh at this point :')

#[derive(Clone)]
pub struct ScratchPadAgent {
    _symbol_event_message_properties: SymbolEventMessageProperties,
    _sender: UnboundedSender<SymbolEventMessage>,
}

impl ScratchPadAgent {
    pub fn new(
        symbol_event_message_properties: SymbolEventMessageProperties,
        sender: UnboundedSender<SymbolEventMessage>,
    ) -> Self {
        Self {
            _symbol_event_message_properties: symbol_event_message_properties,
            _sender: sender,
        }
    }
}

impl ScratchPadAgent {
    /// We try to contain all the events which are coming in from the symbol
    /// which is being edited by the user, the real interface here will look like this
    pub async fn process_envrionment(
        self,
        mut stream: Pin<Box<dyn Stream<Item = EnvironmentEventType> + Send + Sync>>,
    ) {
        println!("scratch_pad_agent::start_processing_environment");
        while let Some(event) = stream.next().await {
            match event {
                EnvironmentEventType::LSP(_lsp_signal) => {
                    // process the lsp signal over here
                }
                EnvironmentEventType::Human(_message) => {
                    // whenever the human sends a request over here, encode it and try
                    // to understand how to handle it, some might require search, some
                    // might be more automagic
                }
                EnvironmentEventType::Symbol(_symbol_event) => {
                    // we know a symbol is going to be edited, what should we do about it?
                }
                EnvironmentEventType::ShutDown => {
                    println!("scratch_pad_agent::shut_down");
                    break;
                }
            }
        }
    }
}
