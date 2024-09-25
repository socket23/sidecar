use uuid::Uuid;

use super::plan_step::PlanStep;

#[derive(Debug, Clone)]
pub struct Plan {
    steps: Vec<PlanStep>,
    initial_context: String, // needs to be a richer type
    user_query: String, // this may only be useful for initial plan generation. Steps better represent the overall direction?
    checkpoint: usize,
}

impl Plan {
    pub fn new(initial_context: String, user_query: String, steps: &[PlanStep]) -> Self {
        let mut plan = Plan {
            steps: steps.to_vec(),
            initial_context,
            user_query,
            checkpoint: 0,
        };
        plan
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
