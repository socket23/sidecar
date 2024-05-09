//! Contains a lock on the different symbols and maintains them running in memory
//! this way we are able to manage different symbols and their run-time while running
//! them in a session.

use std::{collections::HashMap, sync::Arc};

use futures::lock::Mutex;
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::tool::broker::ToolBroker;

use super::{
    identifier::{MechaCodeSymbolThinking, SymbolIdentifier},
    tool_box::ToolBox,
    types::{Symbol, SymbolEvent, SymbolEventRequest, SymbolEventResponse},
};

#[derive(Clone)]
pub struct SymbolLocker {
    symbols: Arc<
        Mutex<
            HashMap<
                // TODO(skcd): what should be the key here for this to work properly
                // cause we can have multiple symbols which share the same name
                // this probably would not happen today but would be good to figure
                // out at some point
                SymbolIdentifier,
                // this is the channel which we use to talk to this particular symbol
                // and everything related to it
                UnboundedSender<(
                    SymbolEvent,
                    tokio::sync::oneshot::Sender<SymbolEventResponse>,
                )>,
            >,
        >,
    >,
    // this is the main communication channel which we can use to send requests
    // to the right symbol
    hub_sender: UnboundedSender<(
        SymbolEventRequest,
        tokio::sync::oneshot::Sender<SymbolEventResponse>,
    )>,
    tools: Arc<ToolBox>,
}

impl SymbolLocker {
    pub fn new(
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
        tools: Arc<ToolBox>,
    ) -> Self {
        Self {
            symbols: Arc::new(Mutex::new(HashMap::new())),
            hub_sender,
            tools,
        }
    }

    pub async fn process_request(
        &self,
        request_event: (
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        ),
    ) {
        let request = request_event.0;
        let sender = request_event.1;
        // we will send the response in this sender
    }

    pub async fn create_symbol_agent(&self, request: MechaCodeSymbolThinking) {
        // say we create the symbol agent, what happens next
        // the agent can have its own events which it might need to do, including the
        // followups or anything else
        // the user might have some events to send
        // other agents might also want to talk to it for some information
        let symbol_identifier = request.to_symbol_identifier();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<(
            SymbolEvent,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>();
        {
            let mut symbols = self.symbols.lock().await;
            symbols.insert(symbol_identifier.clone(), sender);
        }

        // now we create the symbol and let it rip
        let symbol = Symbol::new(
            symbol_identifier,
            request,
            self.hub_sender.clone(),
            self.tools.clone(),
        );

        // now we let it rip, we give the symbol the receiver and ask it
        // to go crazy with it
        tokio::spawn(async move { symbol.run(receiver).await });
        // fin
    }
}
