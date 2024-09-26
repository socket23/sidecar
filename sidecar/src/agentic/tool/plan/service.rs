use std::sync::Arc;

use thiserror::Error;

use crate::{
    agentic::tool::{
        broker::ToolBroker, errors::ToolError, input::ToolInput,
        plan::generator::StepGeneratorRequest, r#type::Tool,
    },
    user_context::types::UserContext,
};

use super::{
    plan::Plan,
    plan_step::{PlanStep, StepExecutionContext},
};

/// Operates on Plan
pub struct PlanService {
    tool_broker: Arc<ToolBroker>,
}

impl PlanService {
    pub fn new(tool_broker: Arc<ToolBroker>) -> Self {
        Self { tool_broker }
    }

    pub async fn create_plan(
        &self,
        query: String,
        user_context: UserContext,
        request_id: String,
        editor_url: String,
    ) -> Result<Plan, ServiceError> {
        let step_generator_request =
            StepGeneratorRequest::new(query.to_owned(), request_id, editor_url)
                .with_user_context(&user_context);

        let plan_steps = self
            .tool_broker
            .invoke(ToolInput::GenerateStep(step_generator_request))
            .await?
            .step_generator_output()
            .ok_or(ServiceError::WrongToolOutput())?
            .into_plan_steps();

        Ok(Plan::new(
            "Placeholder Title (to be computed)".to_owned(),
            "".to_owned(),
            query,
            plan_steps,
        ))
    }

    pub async fn update_plan(&self, plan: Plan, update_query: String) -> Result<(), ServiceError> {
        todo!()
    }

    pub fn step_execution_context(
        &self,
        steps: &[PlanStep],
        index: usize,
    ) -> Vec<StepExecutionContext> {
        let steps_to_now = &steps[..index];

        let context_to_now = steps_to_now
            .iter()
            .map(|step| StepExecutionContext::from(step))
            .collect::<Vec<_>>();

        context_to_now
    }

    pub async fn execute_step(&self, plan: Plan, index: usize) -> Result<(), ServiceError> {
        let context = self.step_execution_context(plan.steps(), index);
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Tool Error: {0}")]
    ToolError(#[from] ToolError),

    #[error("Wrong tool output")]
    WrongToolOutput(),
}
