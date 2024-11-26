//! The selector is the module which selectes the best node to use in a given search
//! tree

use std::sync::Arc;

use crate::mcts::action_node::{ActionNode, SearchTree};

/// Parameters for configuring the behavior of the Selector in UCT score calculations.
pub struct Selector {
    /// Weight factor for the exploitation term in the UCT score calculation.
    /// Higher values favor exploitation over exploration.
    exploitation_weight: f32,

    /// If true, uses the average reward across the trajectory for exploitation
    /// calculation instead of the node reward.
    use_average_reward: bool,

    /// Weight factor for the exploration term in the UCT score calculation.
    /// Higher values encourage more exploration of less-visited nodes.
    exploration_weight: f32,

    /// Weight factor for the depth-based components in the UCT score.
    /// Affects both the depth bonus and penalty calculations.
    depth_weight: f32,

    /// Factor used in calculating the depth bonus.
    /// Higher values increase the bonus for exploring deeper nodes, especially near the root.
    depth_bonus_factor: f32,

    /// Threshold for considering a node's reward as "high value."
    /// Used in various bonus calculations.
    high_value_threshold: f32,

    /// Threshold for considering a node's reward as "low value."
    /// Used in various penalty calculations.
    low_value_threshold: f32,

    /// Threshold for considering a node's reward as "very high value."
    /// Used in the high value child penalty calculation.
    very_high_value_threshold: f32,

    /// Constant bonus applied to high-value leaf nodes to encourage their exploration.
    high_value_leaf_bonus_constant: f32,

    /// Constant used in calculating the bonus for high-value nodes with low-value children,
    /// encouraging "auto-correction."
    high_value_bad_children_bonus_constant: f32,

    /// Constant used in penalizing nodes with very high-value children
    /// to prevent over-exploitation of a single path.
    high_value_child_penalty_constant: f32,

    /// Penalty applied to nodes on a trajectory that has already finished with a high reward,
    /// discouraging revisiting completed paths.
    finished_trajectory_penalty: f32,

    /// Bonus applied to nodes expecting correction, prioritizing exploration of potential fix paths.
    expect_correction_bonus: f32,

    /// List of action types to check for when calculating the high-value bad children bonus.
    check_for_bad_child_actions: Vec<String>,

    /// Weight factor for the diversity bonus.
    /// Higher values increase the bonus for nodes with low similarity to other explored nodes.
    diversity_weight: f32,

    /// Constant used in penalizing nodes that have duplicate children.
    /// Penalty increases with each duplicate.
    duplicate_child_penalty_constant: f32,

    /// Constant used in penalizing nodes that have siblings with the same action name.
    duplicate_action_penalty_constant: f32,
}

impl Selector {
    /// Calculate the exploitation component of the UCT score.
    ///
    /// Purpose: Favors nodes with higher rewards, encouraging the algorithm to exploit
    /// known good paths in the search tree.
    pub fn calculate_exploitation(&self, node_index: usize, graph: &SearchTree) -> f32 {
        let reward = if self.use_average_reward {
            graph.calculate_mean_reward(node_index)
        } else {
            let node_present = graph.get_node(node_index);
            match node_present {
                Some(node) => node
                    .reward()
                    .map(|reward| reward.value() as f32)
                    .unwrap_or(0.0),
                None => 0.0,
            }
        };
        reward
    }

    /// Calculate the exploration component of the UCT score.
    /// Purpose: Encourages the exploration of less-visited nodes, ensuring a balance
    /// between exploitation and exploration in the search process.
    pub fn calculate_exploration(&self, node_index: usize, graph: &SearchTree) -> f32 {
        graph.calculate_exploration(node_index, self.exploration_weight)
    }

    /// Calculate the depth-based exploration bonus.
    /// Purpose: Provides an incentive to explore deeper into the search tree,
    /// particularly for nodes near the root, to encourage thorough exploration.
    pub fn calculate_depth_bonus(&self, node_index: usize, graph: &SearchTree) -> f32 {
        graph.calculate_depth_bonus(node_index, self.depth_bonus_factor, self.depth_weight)
    }

    // TODO(skcd): Pick up the remaining similarity metric work over here and try to rawdog and finish
    // it all up
}
