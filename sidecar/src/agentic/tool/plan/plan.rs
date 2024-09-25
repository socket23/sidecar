use uuid::Uuid;

use crate::user_context::types::UserContext;

use super::plan_step::PlanStep;

#[derive(Debug, Clone)]
pub struct Plan {
    id: Uuid,
    name: String, // for UI label
    steps: Vec<PlanStep>,
    initial_context: String, // this is here for testing, until we have better idea of what input context looks like
    user_context: Option<UserContext>, // originally provided user_context - may or may not be provided
    user_query: String, // this may only be useful for initial plan generation. Steps better represent the overall direction?
    checkpoint: usize,
}

impl Plan {
    pub fn new(
        name: String,
        initial_context: String,
        user_query: String,
        steps: &[PlanStep],
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            user_context: None,
            steps: steps.to_vec(),
            initial_context,
            user_query,
            checkpoint: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn with_user_context(mut self, user_context: &UserContext) -> Self {
        self.user_context = Some(user_context.to_owned());
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

    pub fn edit_step(&mut self, step_id: Uuid, new_content: String) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id() == step_id) {
            step.edit_description(new_content);
        }
    }

    pub fn steps(&self) -> &[PlanStep] {
        &self.steps.as_slice()
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
}
