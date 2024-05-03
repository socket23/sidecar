use std::pin::Pin;

use futures::channel::mpsc::UnboundedReceiver;
use futures::Stream;
use futures::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::action::types::Action;

pub struct Runner {
    action_receiver: UnboundedReceiver<Action>,
}

// We have a single runner here for the current interaction which can have multiple
// events coming in from different mechas or the human, how do we keep running it
// in the background properly (cause there might be error correction loops etc)
// this almost always works on an input stream like fashion to keep things chugging
// along in parallel

// what kind of actions could we have:
// - expose the whole state/show incremental changes?
// - when the user asks a question to any part of the mecha's action, the mecha should respond

pub enum RunnableInteraction {
    HumanInteraction,
}

pub enum AgentEventEmitter {}

impl Runner {
    pub async fn run(
        &mut self,
        // we also need a sender which can send over events as and when required
        mut sender: UnboundedSender<AgentEventEmitter>,
        mut input_stream: Pin<Box<dyn Stream<Item = RunnableInteraction> + Send + Sync>>,
    ) {
        tokio::select! {
            // whatever we get on the run stream might be an action or otherwise
            Some(input) = input_stream.next() => {
                unimplemented!();
            }
            Some(action) = self.action_receiver.next() => {
                // we want to run the action here, by passing it to the action
                // graph and letting it execute
                unimplemented!();
            }
        }
    }
}
