//! Contains a lock on the different symbols and maintains them running in memory
//! this way we are able to manage different symbols and their run-time while running
//! them in a session.
//! Symbol locker has access to the whole fs file-system and can run searches
//! if the file path is not correct or incorrect, cause we have so much information
//! over here, if the symbol is properly defined we are sure to find it, even if there
//! are multiples we have enough context here to gather the information required
//! to create the correct symbol and send it over

use std::{collections::HashMap, sync::Arc};

use futures::lock::Mutex;
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::tool::broker::ToolBroker;

use super::{
    errors::SymbolError,
    events::types::SymbolEvent,
    identifier::{LLMProperties, MechaCodeSymbolThinking, SymbolIdentifier},
    tool_box::ToolBox,
    types::{Symbol, SymbolEventRequest, SymbolEventResponse},
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
    llm_properties: LLMProperties,
}

impl SymbolLocker {
    pub fn new(
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
        tools: Arc<ToolBox>,
        llm_properties: LLMProperties,
    ) -> Self {
        Self {
            symbols: Arc::new(Mutex::new(HashMap::new())),
            hub_sender,
            tools,
            llm_properties,
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
        let symbol_identifier = request.symbol();
        let event = request.event();
        // the events do not matter as much as finding the correct symbol
        // to send to for the request
        // we will send the response in this sender
        // your job is to strictly forward the request to the right symbol
        // or find one if it does not exist at the location we are talking about
        match event {
            SymbolEvent::AskQuestion(ask_question) => {
                todo!("we have to implement this")
            }
            SymbolEvent::Edit(edit_operation) => {}
            SymbolEvent::Delete => {}
            SymbolEvent::InitialRequest => {}
            SymbolEvent::Outline => {
                todo!("we have to implement this")
            }
            SymbolEvent::UserFeedback => {}
        }
    }

    pub async fn create_symbol_agent(
        &self,
        request: MechaCodeSymbolThinking,
    ) -> Result<(), SymbolError> {
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
            self.llm_properties.clone(),
        )
        .await?;

        // now we let it rip, we give the symbol the receiver and ask it
        // to go crazy with it
        tokio::spawn(async move {
            let _ = symbol.run(receiver).await;
        });
        // fin
        Ok(())
    }
}
