//! Contains the UCT score (Upper confidence bounds applied to Trees) score is tracked
//! over here

use std::fmt::Debug;

pub struct UCTScore {
    final_score: f32,
    exploitation: f32,
    exploration: f32,
    depth_bonus: f32,
    depth_penalty: f32,
    high_value_leaf_bonus: f32,
    high_value_bad_children_bonus: f32,
    high_value_child_penalty: f32,
    high_value_parent_bonus: f32,
    finished_trajectory_penalty: f32,
    expect_correction_bonus: f32,
    diversity_bonus: f32,
    duplicate_child_penalty: f32,
    duplicate_action_penalty: f32,
}

impl Debug for UCTScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let components = vec![
            format!("Final Score: {}", self.final_score),
            format!("Exploitation: {}", self.exploitation),
            format!("Exploration: {}", self.exploration),
            format!("Depth Bonus: {}", self.depth_bonus),
            format!("Depth Penalty: {}", self.depth_penalty),
            format!("High Value Leaf Bonus: {}", self.high_value_leaf_bonus),
            format!(
                "High Value Bad Children Bonus: {}",
                self.high_value_bad_children_bonus
            ),
            format!(
                "High Value Child Penalty: {}",
                self.high_value_child_penalty
            ),
            format!("High Value Parent Bonus: {}", self.high_value_parent_bonus),
            format!(
                "Finished Trajectory Penalty: {}",
                self.finished_trajectory_penalty
            ),
            format!("Expect Correction Bonus: {}", self.expect_correction_bonus),
            format!("Diversity Bonus: {}", self.diversity_bonus),
            format!("Duplicate Child Penalty: {}", self.duplicate_child_penalty),
            format!(
                "Duplicate Action Penalty: {}",
                self.duplicate_action_penalty
            ),
        ];
        write!(f, "{}", components.join("\n"))
    }
}
