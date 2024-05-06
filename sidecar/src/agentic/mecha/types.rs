use std::sync::Arc;

use futures::channel::mpsc::UnboundedSender;
use futures::SinkExt;

use crate::agentic::{
    action::types::Action,
    memory::base::Memory,
    tool::{base::Tool, input::ToolInput, output::ToolOutput},
};

use super::journal::MechaJournal;

pub enum MechaState {
    Working,
    Finished,
    Errored,
    Waiting,
}

pub enum MechaEvent {
    UserInteraction(String),
    MechaAction,
}

pub struct Mecha {
    current_task: String,
    id: i64,
    memory: Memory,
    // need to figure this out, cause the mecha might need to take more actions
    // or it is wainting on the execution of an action, how do we codify this
    // properly
    action_history: Vec<Action>,
    state: MechaState,
    sender: UnboundedSender<MechaEvent>,
}

struct MechaToolUsage {
    tool_input: ToolInput,
    tool: Arc<Box<dyn Tool + Sync + Send>>,
}

impl Mecha {
    pub fn new(
        id: i64,
        current_task: String,
        memory: Memory,
        sender: UnboundedSender<MechaEvent>,
    ) -> Self {
        Self {
            id,
            current_task,
            memory,
            action_history: vec![],
            state: MechaState::Working,
            sender,
        }
    }

    fn apply_journal(&mut self, journal: MechaJournal) {
        todo!("we want to apply the journal over here and generate the data");
    }

    fn check_stop_condition(&self) -> bool {
        todo!("not implemented yet");
    }

    // we need a runtime event we can send, saying that the agent wants and needs to iterate
    pub async fn run(&mut self, event: MechaEvent) {
        // do something with the event over here, probably we pass it to the decision of using a tool
        let tool_to_use = self.decide_tool(event);
        if self.check_stop_condition() {
            todo!("finish this up properly cause we want to do something after we stop");
            // send stop notification to the main loop so others can handle it
        }
        let event = self.use_tool(tool_to_use).await;
        // next we might have to take one more action, so we send it back to the
        // event
        if let Some(event) = event {
            self.sender.send(event);
        }
        todo!("Here we move the state of the mecha based on the user context if present or based on its own needs");
    }

    async fn use_tool(&mut self, mecha_tool_input: MechaToolUsage) -> Option<MechaEvent> {
        // The agent might be looking to use a tool
        let tool = mecha_tool_input.tool;
        let tool_input = mecha_tool_input.tool_input;
        // let tool_output = tool.invoke(tool_input, tool_context).await;
        // once we have the output might have to modify our state according to the context
        // self.update_state(&tool_output);
        // now we might have to take another action here or we might be finished
        todo!("finish this up properly");
    }

    fn decide_tool(&mut self, event: MechaEvent) -> MechaToolUsage {
        let mut memory = &mut self.memory;
        todo!("Figure out how to get the tools over here");
    }

    fn update_state(&mut self, tool_output: &ToolOutput) {
        todo!("update the state here after looking at the tool output");
    }
}
