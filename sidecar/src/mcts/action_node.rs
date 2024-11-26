use std::collections::HashMap;

use crate::agentic::tool::input::ToolInputPartial;

use super::value_function::reward::Reward;

pub struct ActionObservation {
    _message: String,
    _summary: Option<String>,
    _terminal: bool,
    _expect_correction: bool,
}

pub struct ActionNode {
    index: usize,
    _action: Option<ToolInputPartial>,
    _feedback: Option<String>,
    _is_duplicate: bool,
    reward: Option<Reward>,
    visits: u32,
    value: f32,
    _max_expansions: usize,
}

impl ActionNode {
    pub fn reward(&self) -> Option<&Reward> {
        self.reward.as_ref()
    }
}

pub struct SearchTree {
    pub index_to_node: HashMap<usize, ActionNode>,
    _node_to_children: HashMap<usize, Vec<usize>>,
    node_to_parent: HashMap<usize, Option<usize>>,
}

impl SearchTree {
    fn parent(&self, node: &ActionNode) -> Option<&ActionNode> {
        if let Some(Some(parent_index)) = self.node_to_parent.get(&node.index) {
            self.index_to_node.get(parent_index)
        } else {
            None
        }
    }

    pub fn get_node(&self, node_index: usize) -> Option<&ActionNode> {
        self.index_to_node.get(&node_index)
    }

    fn _children<'a>(
        &'a self,
        node: &ActionNode,
    ) -> Option<impl Iterator<Item = &ActionNode> + 'a> {
        self._node_to_children
            .get(&node.index)
            .map(move |child_indices| {
                child_indices
                    .iter()
                    .filter_map(move |idx| self.index_to_node.get(idx))
            })
    }

    fn get_root<'a>(&'a self, node: &'a ActionNode) -> &'a ActionNode {
        let mut current_node = node;
        while let Some(parent_node) = self.parent(current_node) {
            current_node = parent_node;
        }
        current_node
    }

    fn add_child(&mut self, parent_index: usize, child: ActionNode) {
        let child_index = child.index;
        self.index_to_node.insert(child_index, child);
        self._node_to_children
            .entry(parent_index)
            .or_insert_with(Vec::new)
            .push(child_index);
        self.node_to_parent.insert(child_index, Some(parent_index));
    }

    /// Creates the mean reward on the trajectory over here by traversing the tree
    pub fn calculate_mean_reward(&self, node_index: usize) -> f32 {
        let mut node = self.index_to_node.get(&node_index);
        let mut rewards: Vec<f32> = vec![];
        while node.is_some() {
            let expected_node = node.expect("is_some to hold");
            // add the reward
            rewards.push(if expected_node.visits > 0 {
                expected_node.value / (expected_node.visits as f32)
            } else {
                0.0
            });

            // move up the tree
            node = self.parent(expected_node);
        }

        if rewards.is_empty() {
            0.0
        } else {
            let rewards_len = rewards.len();
            let rewards_sum: f32 = rewards.into_iter().sum();
            rewards_sum / (rewards_len as f32)
        }
    }

    pub fn calculate_exploration(&self, node_index: usize, exploration_weight: f32) -> f32 {
        // Retrieve the current node
        let node = self
            .get_node(node_index)
            .expect("Node index should be valid");

        // Retrieve the parent visits
        let parent_visits = if let Some(parent_node) = self.parent(node) {
            parent_node.visits as f32
        } else {
            1.0 // Default to 1.0 if there's no parent
        };

        // Retrieve the current node's visits
        let node_visits = node.visits as f32;

        if node_visits == 0.0 {
            f32::INFINITY // Favor exploration of unvisited nodes
        } else {
            exploration_weight * ((parent_visits.ln() / node_visits).sqrt())
        }
    }

    pub fn get_depth(&self, node_index: usize) -> u32 {
        let mut depth = 0;
        let mut node = self.get_node(node_index);
        while node.is_some() {
            let expected_node = node.expect("is_some to hold");
            node = self.parent(expected_node);
            if node.is_some() {
                depth = depth + 1;
            }
        }
        depth
    }

    pub fn calculate_depth_bonus(
        &self,
        node_index: usize,
        depth_bonus_factor: f32,
        depth_weight: f32,
    ) -> f32 {
        // Get the depth of the current node
        let depth = self.get_depth(node_index) as f32;

        // Calculate the depth-based bonus
        if depth == 0.0 {
            depth_bonus_factor * (-depth_weight * (depth - 1.0)).exp()
        } else {
            0.0
        }
    }
}
