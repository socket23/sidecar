use std::sync::Arc;

use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    agentic::{
        symbol::{
            identifier::{LLMProperties, SymbolIdentifier},
            ui_event::UIEventWithID,
        },
        tool::{
            broker::ToolBroker, code_edit::search_and_replace::SearchAndReplaceEditingRequest,
            errors::ToolError, input::ToolInput, plan::generator::StepGeneratorRequest,
            r#type::Tool,
        },
    },
    chunking::text_document::Range,
    user_context::types::UserContext,
};

use super::{
    plan::Plan,
    plan_step::{PlanStep, StepExecutionContext},
};

/// Operates on Plan
pub struct PlanService {
    tool_broker: Arc<ToolBroker>,
    llm_properties: LLMProperties,
    ui_sender: UnboundedSender<UIEventWithID>,
    editor_url: String,
}

impl PlanService {
    pub fn new(
        tool_broker: Arc<ToolBroker>,
        llm_properties: LLMProperties,
        ui_sender: UnboundedSender<UIEventWithID>,
        editor_url: String,
    ) -> Self {
        Self {
            tool_broker,
            llm_properties,
            ui_sender,
            editor_url,
        }
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
        )
        .with_user_context(user_context))
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

    pub async fn execute_step(
        &self,
        plan: Plan,
        index: usize,
        root_request_id: String,
    ) -> Result<(), ServiceError> {
        let steps = plan.steps();
        let step_to_execute = steps.get(index).ok_or(ServiceError::StepNotFound(index))?;
        let context = self.step_execution_context(steps, index);

        let fs_file_path = step_to_execute.file_to_edit();

        let request = SearchAndReplaceEditingRequest::new(
            fs_file_path,
            Range::default(),
            "context_in_edit_selection".to_owned(),
            "complete_file".to_owned(),
            "extra_data".to_owned(),
            self.llm_properties.clone(),
            None,
            "instructions".to_owned(),
            root_request_id,
            SymbolIdentifier::with_file_path("symbol_name", "fs_file_path"),
            "edit_request_id".to_owned(),
            self.ui_sender.clone(),
            None,
            self.editor_url.clone(),
            None,
            vec![],
            vec![],
            false,
        );

        let response = self
            .tool_broker
            .invoke(ToolInput::SearchAndReplaceEditing(request))
            .await?;

        dbg!(&response);

        todo!()
    }
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Tool Error: {0}")]
    ToolError(#[from] ToolError),

    #[error("Wrong tool output")]
    WrongToolOutput(),

    #[error("Step not found: {0}")]
    StepNotFound(usize),
}
