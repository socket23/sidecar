//! Contains the action graph here and the execution engine
//! Each action can either run independently or might spawn other actions
//! or depend on it, we need a way to run this properly overe here in an execution
//! loop with some degree of parallelism
//! also any updates made by the human are also tagged as actions and are stored
//! here, this can come in handy when influencing some actions
//! there is also a stop trigger if the human decides to do something and then
//! takes over

use futures::SinkExt;

use crate::agentic::action::types::Action;
use futures::channel::mpsc::UnboundedSender;
use petgraph::{graph::DiGraph, visit::IntoNodeIdentifiers};

pub type NodeIndex = petgraph::graph::NodeIndex<u32>;

pub struct ActionGraph {
    graph: DiGraph<Action, ()>,
    sender: UnboundedSender<NodeIndex>,
}

impl ActionGraph {
    pub fn new(sender: UnboundedSender<NodeIndex>) -> Self {
        ActionGraph {
            graph: DiGraph::new(),
            sender,
        }
    }

    fn add_action(&mut self, action: Action) -> NodeIndex {
        self.graph.add_node(action)
    }

    fn contains_action(&self, action: &Action) -> bool {
        self.graph
            .node_identifiers()
            .any(|node_identifier| &node_identifier == action.id())
    }

    fn add_dependency(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, ());
    }

    pub fn update_action(&mut self, action: Action) -> NodeIndex {
        if self.contains_action(&action) {
            self.update_action(action);
        } else {
            self.add_action(action);
        }
        unimplemented!();
    }

    // TODO(skcd): We just poll this somehow
    pub async fn execute_action(&mut self, action: NodeIndex) {
        // so here the action could add more dependencies or keep running until
        // completion, we check that and make it work somehow
        let action = self.graph.node_weight_mut(action);
        let actions = match action {
            Some(action) => action.run().await,
            None => vec![],
        };
        // now we update our graph with these actions, and send updates about
        // those which are complete
        // first we update all the actions and then get back their output
        // over here
        let updated_action_ids = actions
            .into_iter()
            .map(|action| self.update_action(action))
            .collect::<Vec<_>>();
        // we send over the id of the actions we want to perform forward again
        updated_action_ids
            .into_iter()
            .for_each(|updated_action_id| {
                let _ = self.sender.send(updated_action_id);
            });
    }
}
