use uuid::Uuid;

use super::plan_step::PlanStep;

#[derive(Debug)]
pub struct Plan {
    steps: Vec<PlanStep>,
    initial_context: String,
    user_query: String,
}

impl Plan {
    pub fn new(initial_context: String, user_query: String) -> Self {
        let mut plan = Plan {
            steps: Vec::new(),
            initial_context,
            user_query,
        };
        // plan.generate_steps();
        plan
    }

    // fn generate_steps(&mut self) {
    //     // Placeholder for step generation logic
    //     // In practice, this might involve parsing the user query and initial context
    //     self.steps
    //         .push(PlanStep::new("Initialize the project".to_string()));
    //     self.steps
    //         .push(PlanStep::new("Set up the main function".to_string()));
    // }

    pub fn edit_step(&mut self, step_id: Uuid, new_content: String) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id() == step_id) {
            step.edit_content(new_content);
        }
    }

    pub fn add_context_to_step(&mut self, step_id: Uuid, new_context: String) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id() == step_id) {
            step.add_context(new_context);
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
}
