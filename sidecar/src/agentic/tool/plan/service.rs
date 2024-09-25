use std::sync::Arc;

use thiserror::Error;

use crate::{
    agentic::{
        symbol::errors::SymbolError,
        tool::{
            broker::ToolBroker, errors::ToolError, input::ToolInput,
            plan::generator::StepGeneratorRequest, r#type::Tool,
        },
    },
    user_context::types::UserContext,
};

use super::plan::Plan;

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
        let step_generator_request = StepGeneratorRequest::new(query, request_id, editor_url)
            .with_user_context(&user_context);

        let response = self
            .tool_broker
            .invoke(ToolInput::GenerateStep(step_generator_request))
            .await?
            .step_generator_output()
            .ok_or(ServiceError::WrongToolOutput())?;

        todo!()
    }

    pub async fn update_plan(&self, plan: Plan, update_query: String) -> Result<(), ServiceError> {
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
