use crate::agentic::memory::base::Memory;

use super::graph::NodeIndex;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ActionState {
    OnGoing,
    Finished,
    Waiting,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Action {
    id: NodeIndex,
    memory: Memory,
    state: ActionState,
}

impl Action {
    pub fn id(&self) -> &NodeIndex {
        &self.id
    }
    pub async fn run(&mut self) -> Vec<Self> {
        todo!("we need to make it return the action states after this");
    }
}
