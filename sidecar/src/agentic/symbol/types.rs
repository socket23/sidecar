//! Contains the symbol type and how its structred and what kind of operations we
//! can do with it, its a lot of things but the end goal is that each symbol in the codebase
//! can be represented by some entity, and that's what we are storing over here
//! Inside each symbol we also have the various implementations of it, which we always
//! keep track of and whenever a question is asked we forward it to all the implementations
//! and select the ones which are necessary.

use std::sync::Arc;

use derivative::Derivative;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::agentic::tool::broker::ToolBroker;

use super::identifier::{MechaCodeSymbolThinking, SymbolIdentifier};

pub enum SymbolEvent {
    Create,
    AskQuestion,
    UserFeedback,
    Delete,
    Edit,
}

pub struct SymbolEventRequest {
    symbol: String,
    event: SymbolEvent,
}

pub struct SymbolEventResponse {}

/// The symbol is going to spin in the background and keep working on things
/// is this how we want it to work???
/// ideally yes, cause its its own process which will work in the background
#[derive(Derivative)]
#[derivative(PartialEq, Eq, Debug)]
pub struct Symbol {
    symbol_identifier: SymbolIdentifier,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    hub_sender: UnboundedSender<(
        SymbolEventRequest,
        tokio::sync::oneshot::Sender<SymbolEventResponse>,
    )>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    tools: Arc<ToolBroker>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    mecha_code_symbol: MechaCodeSymbolThinking,
}

impl Symbol {
    pub fn new(
        symbol_identifier: SymbolIdentifier,
        mecha_code_symbol: MechaCodeSymbolThinking,
        // this can be used to talk to other symbols and get them
        // to act on certain things
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
        tools: Arc<ToolBroker>,
    ) -> Self {
        Self {
            mecha_code_symbol,
            symbol_identifier,
            hub_sender,
            tools,
        }
    }

    /// Code selection logic for the symbols here
    fn generate_initial_request(&self) -> MechaCodeSymbolThinking {
        if let Some(snippet) = self.mecha_code_symbol.get_snippet() {
            // We need to provide the agent with all the implementations and ask
            // it what changes will be required at each of these places
            // operation used: rerank + select the most relevant over here
            // we have to figure out how to select the best ones over here
            // maybe we do a rolling window and yes and no if we have to edit
            // or we can do no changes and still execute the query
        } else {
            // we have to figure out the location for this symbol and understand
            // where we want to put this symbol at
            // what would be the best way to do this?
            // should we give the folder overview and then ask it
            // or assume that its already written out
        }
        unimplemented!();
    }

    pub async fn run(
        self,
        mut receiver: UnboundedReceiver<(
            SymbolEvent,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
    ) {
        // we can send the first request to the hub and then poll it from
        // the receiver over here
        // TODO(skcd): Pick it up from over here
        while let Some(event) = receiver.recv().await {
            // we are going to process the events one by one over here
            // we should also have a shut-down event which the symbol sends to itself
            // we can use the hub sender over here to make sure that forwarding the events
            // work as usual, its a roundabout way of doing it, but should work
            // TODO(skcd): Pick it up from here
        }
    }
}
