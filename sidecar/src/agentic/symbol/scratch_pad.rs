//! Contains the scratch pad agent whose job is to work alongside the developer
//! and help them accomplish a task
//! This way the agent can look at all the events and the requests which are happening
//! and take a decision based on them on what should happen next

use std::{pin::Pin, sync::Arc};

use futures::{stream, Stream, StreamExt};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    agentic::symbol::ui_event::UIEventWithID,
    chunking::text_document::{Position, Range},
};

use super::{
    errors::SymbolError,
    events::{
        edit::{SymbolToEdit, SymbolToEditRequest},
        environment_event::EnvironmentEventType,
        human::{HumanAnchorRequest, HumanMessage},
        message_event::{SymbolEventMessage, SymbolEventMessageProperties},
        types::SymbolEvent,
    },
    identifier::SymbolIdentifier,
    tool_box::ToolBox,
    tool_properties::ToolProperties,
    types::{SymbolEventRequest, SymbolEventResponse},
};

/// Different kind of events which can happen
/// We should move beyond symbol events tbh at this point :')

#[derive(Clone)]
pub struct ScratchPadAgent {
    storage_fs_path: String,
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
            storage_fs_path: "/Users/skcd/test_repo/sidecar/scratchpad.md".to_owned(),
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

        let cloned_self = self.clone();
        let cloned_user_query = anchor_request.user_query().to_owned();
        let _ = tokio::spawn(async move {
            cloned_self.mark_start_of_iteration(cloned_user_query).await;
        });

        let edits_done = stream::iter(symbols_to_edit_request.into_iter().map(|data| {
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
        .await
        .into_iter()
        .filter_map(|s| s.ok())
        .collect::<Vec<_>>();

        let cloned_self = self.clone();
        let cloned_user_query = anchor_request.user_query().to_owned();
        let _ = tokio::spawn(async move {
            let _ = cloned_self
                .react_to_edits(edits_done, cloned_user_query)
                .await;
        });
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

    async fn mark_start_of_iteration(&self, user_query: String) {
        println!("scratch_pad::mark_start_of_iteration");
        let scratch_pad = self.storage_fs_path.to_owned();
        let symbols_to_edit_request = SymbolToEditRequest::new(vec![SymbolToEdit::new(
            scratch_pad.to_owned(),
            Range::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            scratch_pad.to_owned(),
            vec![format!(r#"Record your start on the following user query:
{user_query}"#)],
            false,
            false,
            true,
            "Your job is to track the user query in your scratch pad, this way you are able to keep track of the work you are going to do".to_owned(),
            None,
            false,
            None,
            false,
        )], SymbolIdentifier::with_file_path(&scratch_pad, &scratch_pad), vec![]);
        let (sender, _) = tokio::sync::oneshot::channel();
        let symbol_event_request = SymbolEventRequest::new(
            symbols_to_edit_request.symbol_identifier().clone(),
            SymbolEvent::Edit(symbols_to_edit_request),
            ToolProperties::new(),
        );
        let event = SymbolEventMessage::message_with_properties(
            symbol_event_request,
            self.message_properties.clone(),
            sender,
        );
        let _ = self.symbol_event_sender.send(event);
    }

    /// We want to react to the various edits which have happened and the request they were linked to
    /// and come up with next steps and try to understand what we can do to help the developer
    async fn react_to_edits(&self, edits: Vec<SymbolEventResponse>, user_query: String) {
        println!("scratch_pad::react_to_edits");
        let after_edits_changes = edits
            .into_iter()
            .map(|symbol_event_response| symbol_event_response.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let scratch_pad = self.storage_fs_path.to_owned();

        let symbols_to_edit_request = SymbolToEditRequest::new(vec![SymbolToEdit::new(
                    scratch_pad.to_owned(),
                    Range::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                    scratch_pad.to_owned(),
                    vec![format!(r#"Record your insights from working on the user query here, use this as a running notepad:
<user_query>
{user_query}
</user_query>
<changes_made>
{after_edits_changes}
</changes_made>"#).to_owned()],
                    false,
                    false,
                    true,
                    "Record your insights from working on the user query here, use this as a running notepad".to_owned(),
                    None,
                    false,
                    None,
                    true,
                )], SymbolIdentifier::with_file_path(&scratch_pad, &scratch_pad), vec![]);
        let (sender, _) = tokio::sync::oneshot::channel();
        let symbol_event_request = SymbolEventRequest::new(
            symbols_to_edit_request.symbol_identifier().clone(),
            SymbolEvent::Edit(symbols_to_edit_request),
            ToolProperties::new(),
        );
        let event = SymbolEventMessage::message_with_properties(
            symbol_event_request,
            self.message_properties.clone(),
            sender,
        );
        let _ = self.symbol_event_sender.send(event);
    }
}
