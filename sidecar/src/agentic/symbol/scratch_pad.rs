//! Contains the scratch pad agent whose job is to work alongside the developer
//! and help them accomplish a task
//! This way the agent can look at all the events and the requests which are happening
//! and take a decision based on them on what should happen next

use std::{pin::Pin, sync::Arc};

use futures::{stream, Stream, StreamExt};
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::symbol::ui_event::UIEventWithID;

use super::{
    errors::SymbolError,
    events::{
        environment_event::EnvironmentEventType,
        human::{HumanAnchorRequest, HumanMessage},
        message_event::{SymbolEventMessage, SymbolEventMessageProperties},
        types::SymbolEvent,
    },
    tool_box::ToolBox,
    tool_properties::ToolProperties,
    types::SymbolEventRequest,
};

/// Different kind of events which can happen
/// We should move beyond symbol events tbh at this point :')

#[derive(Clone)]
pub struct ScratchPadAgent {
    message_properties: SymbolEventMessageProperties,
    tool_box: Arc<ToolBox>,
    symbol_event_sender: UnboundedSender<SymbolEventMessage>,
}

impl ScratchPadAgent {
    pub fn new(
        message_properties: SymbolEventMessageProperties,
        tool_box: Arc<ToolBox>,
        symbol_event_sender: UnboundedSender<SymbolEventMessage>,
    ) -> Self {
        Self {
            message_properties,
            tool_box,
            symbol_event_sender,
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
                EnvironmentEventType::Human(message) => {
                    println!("scratch_pad_agent::human_message::({:?})", &message);
                    let _ = self.handle_human_message(message).await;
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

    async fn handle_human_message(&self, human_message: HumanMessage) -> Result<(), SymbolError> {
        match human_message {
            HumanMessage::Anchor(anchor_request) => self.human_message_anchor(anchor_request).await,
            HumanMessage::Followup(_followup_request) => Ok(()),
        }
    }

    async fn human_message_anchor(
        &self,
        anchor_request: HumanAnchorRequest,
    ) -> Result<(), SymbolError> {
        let start_instant = std::time::Instant::now();
        println!("scratch_pad_agent::human_message_anchor::start");
        let anchored_symbols = anchor_request.anchored_symbols();
        let symbols_to_edit_request = self
            .tool_box
            .symbol_to_edit_request(
                anchored_symbols,
                anchor_request.user_query(),
                anchor_request.anchor_request_context(),
                self.message_properties.clone(),
            )
            .await?;

        let _edits_done = stream::iter(symbols_to_edit_request.into_iter().map(|data| {
            (
                data,
                self.message_properties.clone(),
                self.symbol_event_sender.clone(),
            )
        }))
        .map(
            |(symbol_to_edit_request, message_properties, symbol_event_sender)| async move {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                let symbol_event_request = SymbolEventRequest::new(
                    symbol_to_edit_request.symbol_identifier().clone(),
                    SymbolEvent::Edit(symbol_to_edit_request), // defines event type
                    ToolProperties::new(),
                );
                let event = SymbolEventMessage::message_with_properties(
                    symbol_event_request,
                    message_properties,
                    sender,
                );
                let _ = symbol_event_sender.send(event);
                receiver.await
            },
        )
        // run 100 edit requests in parallel to prevent race conditions
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;
        println!(
            "scratch_pad_agent::human_message_anchor::end::time_taken({}ms)",
            start_instant.elapsed().as_millis()
        );
        // send end of iteration event over here to the frontend
        let _ = self
            .message_properties
            .ui_sender()
            .send(UIEventWithID::code_iteration_finished(
                self.message_properties.request_id_str().to_owned(),
            ));
        Ok(())
    }
}
