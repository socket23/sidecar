use serde::{Deserialize, Serialize};

use crate::user_context::types::UserContext;

use super::plan_step::PlanStep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    id: String,
    name: String, // for UI label
    steps: Vec<PlanStep>,
    initial_context: String, // this is here for testing, until we have better idea of what input context looks like
    user_context: Option<UserContext>, // originally provided user_context - may or may not be provided
    user_query: String, // this may only be useful for initial plan generation. Steps better represent the overall direction?
    checkpoint: usize,
    storage_path: String,
}

impl Plan {
    pub fn new(
        id: String,
        name: String,
        initial_context: String, // todo(zi): consider whether this should be user_context or other.
        user_query: String,
        steps: Vec<PlanStep>,
        storage_path: String,
    ) -> Self {
        Self {
            id,
            name,
            user_context: None,
            steps,
            initial_context,
            user_query,
            checkpoint: 0,
            storage_path,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn with_user_context(mut self, user_context: UserContext) -> Self {
        self.user_context = Some(user_context);
        self
    }

    pub fn user_context(&self) -> Option<&UserContext> {
        self.user_context.as_ref()
    }

    pub fn add_step(&mut self, step: PlanStep) {
        self.steps.push(step);
    }

    pub fn add_steps(&mut self, steps: &[PlanStep]) {
        self.steps.extend(steps.to_vec())
    }

    pub fn edit_step(&mut self, step_id: String, new_content: String) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id() == step_id) {
            step.edit_description(new_content);
        }
    }

    pub fn steps(&self) -> &[PlanStep] {
        &self.steps.as_slice()
    }

    pub fn steps_mut(&mut self) -> &mut Vec<PlanStep> {
        &mut self.steps
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn initial_context(&self) -> &str {
        &self.initial_context
    }

    pub fn user_query(&self) -> &str {
        &self.user_query
    }

    pub fn checkpoint(&self) -> usize {
        self.checkpoint
    }

    pub fn increment_checkpoint(&mut self) {
        self.checkpoint = self.checkpoint.saturating_add(1);
    }

    pub fn set_checkpoint(&mut self, index: usize) {
        self.checkpoint = index;
    }

    pub fn final_checkpoint(&self) -> usize {
        &self.steps.len() - 1
    }
}
