//! Contains the action node which will keep track of the graph like structure
//! for the mcts search

use petgraph::Graph;

use crate::agentic::tool::{input::ToolInputPartial, r#type::ToolType};

use super::value_function::reward::Reward;

pub type ActionNodeIndex = petgraph::graph::NodeIndex<usize>;

pub struct ActionNode {
    /// The index of the node
    index: ActionNodeIndex,
    /// The action associated with this node
    action: Option<ToolInputPartial>,
    /// Feedback provided to the node
    feedback: Option<String>,
    /// Flag to indicate if the node is a duplicate
    is_duplicate: bool,
    /// The reward of the node
    reward: Option<Reward>,
    /// The number of times the node has been visited
    visits: usize,
    /// The total value (reward) of the node
    value: f32,
    /// The maximum number of expansions
    max_expansions: usize,
}

/// Contains the MCTS Tree which contains a reference to the graph and also
/// the root_idx of the node we are interested in
///
/// We can use this tree to freely traverse the search space
pub struct SearchTree {
    pub graph: Graph<ActionNode, ToolType>,
    root_idx: ActionNodeIndex,
}
