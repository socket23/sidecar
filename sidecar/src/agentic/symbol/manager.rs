//! Contains the central manager for the symbols and maintains them in the system
//! as a connected graph in some ways in which these symbols are able to communicate
//! with each other

use std::sync::Arc;

use futures::{stream, StreamExt};
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::tool::base::Tool;
use crate::chunking::editor_parsing::EditorParsing;
use crate::user_context::types::UserContext;
use crate::{
    agentic::tool::{broker::ToolBroker, output::ToolOutput},
    inline_completion::symbols_tracker::SymbolTrackerInline,
};

use super::identifier::LLMProperties;
use super::tool_box::ToolBox;
use super::ui_event::UIEventWithID;
use super::{
    errors::SymbolError,
    events::input::SymbolInputEvent,
    locker::SymbolLocker,
    types::{SymbolEventRequest, SymbolEventResponse},
};

// This is the main communication manager between all the symbols
// this of this as the central hub through which all the events go forward
pub struct SymbolManager {
    sender: UnboundedSender<(
        SymbolEventRequest,
        String,
        tokio::sync::oneshot::Sender<SymbolEventResponse>,
    )>,
    // this is the channel where the various symbols will use to talk to the manager
    // which in turn will proxy it to the right symbol, what happens if there are failures
    // each symbol has its own receiver which is being used
    symbol_locker: SymbolLocker,
    tools: Arc<ToolBroker>,
    symbol_broker: Arc<SymbolTrackerInline>,
    editor_parsing: Arc<EditorParsing>,
    tool_box: Arc<ToolBox>,
    editor_url: String,
    llm_properties: LLMProperties,
    ui_sender: UnboundedSender<UIEventWithID>,
}

impl SymbolManager {
    pub fn new(
        tools: Arc<ToolBroker>,
        symbol_broker: Arc<SymbolTrackerInline>,
        editor_parsing: Arc<EditorParsing>,
        editor_url: String,
        ui_sender: UnboundedSender<UIEventWithID>,
        llm_properties: LLMProperties,
        // This is a hack and not a proper one at that, we obviously want to
        // do better over here
        user_context: UserContext,
    ) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<(
            SymbolEventRequest,
            String,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>();
        let tool_box = Arc::new(ToolBox::new(
            tools.clone(),
            symbol_broker.clone(),
            editor_parsing.clone(),
            editor_url.to_owned(),
            ui_sender.clone(),
        ));
        let symbol_locker = SymbolLocker::new(
            sender.clone(),
            tool_box.clone(),
            llm_properties.clone(),
            user_context,
            ui_sender.clone(),
        );
        let cloned_symbol_locker = symbol_locker.clone();
        let cloned_ui_sender = ui_sender.clone();
        tokio::spawn(async move {
            // TODO(skcd): Make this run in full parallelism in the future, for
            // now this is fine
            while let Some(event) = receiver.recv().await {
                // let _ = cloned_ui_sender.send(UIEvent::from(event.0.clone()));
                let _ = cloned_symbol_locker.process_request(event).await;
            }
        });
        Self {
            sender,
            symbol_locker,
            editor_parsing,
            tools,
            symbol_broker,
            tool_box,
            editor_url,
            llm_properties,
            ui_sender,
        }
    }

    // This is just for testing out the flow for single input events
    pub async fn probe_request(&self, input_event: SymbolEventRequest) -> Result<(), SymbolError> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request_id = uuid::Uuid::new_v4().to_string();
        let _ = self
            .symbol_locker
            .process_request((input_event, request_id, sender))
            .await;
        let response = receiver.await;
        println!("{:?}", response.expect("to work"));
        Ok(())
    }

    // once we have the initial request, which we will go through the initial request
    // mode once, we have the symbols from it we can use them to spin up sub-symbols as well
    pub async fn initial_request(&self, input_event: SymbolInputEvent) -> Result<(), SymbolError> {
        let user_context = input_event.provided_context().clone();
        let request_id = uuid::Uuid::new_v4().to_string();
        self.ui_sender.send(UIEventWithID::for_codebase_event(
            request_id.to_owned(),
            input_event.clone(),
        ));
        let tool_input = input_event.tool_use_on_initial_invocation().await;
        println!("{:?}", &tool_input);
        if let Some(tool_input) = tool_input {
            // send the tool input to the ui sender
            let _ = self.ui_sender.send(UIEventWithID::from_tool_event(
                request_id.to_owned(),
                tool_input.clone(),
            ));
            if let ToolOutput::ImportantSymbols(important_symbols) = self
                .tools
                .invoke(tool_input)
                .await
                .map_err(|e| SymbolError::ToolError(e))?
            {
                println!("{:?}", &important_symbols);
                let symbols = self
                    .tool_box
                    .important_symbols(important_symbols, user_context, &request_id)
                    .await
                    .map_err(|e| e.into())?;
                let request_id_ref = &request_id;
                // This is where we are creating all the symbols
                let _ = stream::iter(symbols)
                    .map(|symbol_request| async move {
                        let _ = self
                            .symbol_locker
                            .create_symbol_agent(symbol_request, request_id_ref.to_owned())
                            .await;
                    })
                    .buffer_unordered(100)
                    .collect::<Vec<_>>()
                    .await;
            }
        } else {
            // We are for some reason not even invoking the first passage which is
            // weird, this is completely wrong and we should be replying back
            println!("this is wrong, please look at the comment here");
        }
        Ok(())
    }
}
