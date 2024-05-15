//! Contains the central manager for the symbols and maintains them in the system
//! as a connected graph in some ways in which these symbols are able to communicate
//! with each other

use std::sync::Arc;

use futures::{stream, StreamExt};
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::tool::base::Tool;
use crate::agentic::tool::errors::ToolError;
use crate::agentic::tool::input::ToolInput;
use crate::chunking::editor_parsing::{self, EditorParsing};
use crate::{
    agentic::tool::{broker::ToolBroker, output::ToolOutput},
    inline_completion::symbols_tracker::SymbolTrackerInline,
};

use super::identifier::LLMProperties;
use super::tool_box::ToolBox;
use super::ui_event::UIEvent;
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
}

impl SymbolManager {
    pub fn new(
        tools: Arc<ToolBroker>,
        symbol_broker: Arc<SymbolTrackerInline>,
        editor_parsing: Arc<EditorParsing>,
        editor_url: String,
        ui_sender: UnboundedSender<UIEvent>,
        llm_properties: LLMProperties,
    ) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<(
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>();
        let tool_box = Arc::new(ToolBox::new(
            tools.clone(),
            symbol_broker.clone(),
            editor_parsing.clone(),
            editor_url.to_owned(),
            ui_sender.clone(),
        ));
        let symbol_locker =
            SymbolLocker::new(sender.clone(), tool_box.clone(), llm_properties.clone());
        let cloned_symbol_locker = symbol_locker.clone();
        let cloned_ui_sender = ui_sender.clone();
        tokio::spawn(async move {
            // TODO(skcd): Make this run in full parallelism in the future, for
            // now this is fine
            while let Some(event) = receiver.recv().await {
                let _ = cloned_ui_sender.send(UIEvent::from(event.0.clone()));
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
        }
    }

    // once we have the initial request, which we will go through the initial request
    // mode once, we have the symbols from it we can use them to spin up sub-symbols as well
    pub async fn initial_request(&self, input_event: SymbolInputEvent) -> Result<(), SymbolError> {
        let tool_input = input_event.tool_use_on_initial_invocation();
        println!("{:?}", &tool_input);
        if let Some(tool_input) = tool_input {
            if let ToolOutput::ImportantSymbols(important_symbols) = self
                .tools
                .invoke(tool_input)
                .await
                .map_err(|e| SymbolError::ToolError(e))?
            {
                let symbols = self
                    .tool_box
                    .important_symbols(important_symbols)
                    .await
                    .map_err(|e| e.into())?;
                // This is where we are creating all the symbols
                let _ = stream::iter(symbols)
                    .map(|symbol_request| async move {
                        let _ = self.symbol_locker.create_symbol_agent(symbol_request).await;
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

    async fn invoke_tool_broker(&self, tool_input: ToolInput) -> Result<ToolOutput, ToolError> {
        self.tools.invoke(tool_input).await
    }
}
