use std::sync::Arc;

use futures::{stream, StreamExt};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    agentic::{
        symbol::{
            errors::SymbolError,
            events::message_event::SymbolEventMessageProperties,
            identifier::{LLMProperties, SymbolIdentifier},
            tool_box::ToolBox,
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
    tool_box: Arc<ToolBox>,
    llm_properties: LLMProperties,
    message_properties: SymbolEventMessageProperties,
}

impl PlanService {
    pub fn new(
        tool_broker: Arc<ToolBroker>,
        tool_box: Arc<ToolBox>,
        llm_properties: LLMProperties,
        message_properties: SymbolEventMessageProperties,
    ) -> Self {
        Self {
            tool_broker,
            tool_box,
            llm_properties,
            message_properties,
        }
    }

    pub async fn create_plan(
        &self,
        query: String,
        user_context: UserContext,
    ) -> Result<Plan, ServiceError> {
        let request_id = self.message_properties.request_id().request_id();
        let editor_url = self.message_properties.editor_url();
        let step_generator_request =
            StepGeneratorRequest::new(query.to_owned(), request_id.to_owned(), editor_url)
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
        root_request_id: String,
    ) -> Result<(), ServiceError> {
        let checkpoint = plan.checkpoint();

        let steps = plan.steps();
        let step_to_execute = steps
            .get(checkpoint)
            .ok_or(ServiceError::StepNotFound(checkpoint))?;
        let contexts = self.step_execution_context(steps, checkpoint);

        // turn all of that into a fat context string...
        let full_context_as_string = stream::iter(contexts.iter().enumerate().map(
            |(index, context)| async move {
                let context_string = context.to_string().await;
                format!("Step {}:\n{}", index + 1, context_string)
            },
        ))
        .buffer_unordered(3)
        .collect::<Vec<_>>()
        .await
        .join("\n");

        // todo(zi) consider accumulating this in a context manager vs recomputing for each step (long)

        let instruction = step_to_execute.description();

        let fs_file_path = step_to_execute.file_to_edit();

        let file_content = self
            .tool_box
            .file_open(fs_file_path.clone(), self.message_properties.clone())
            .await?
            .contents();

        let request = SearchAndReplaceEditingRequest::new(
            fs_file_path.to_owned(),
            Range::default(),
            "".to_owned(),
            file_content,
            full_context_as_string, // todo(zi): consider giving system_prompt more info about this being plan history
            self.llm_properties.clone(),
            None,
            instruction.to_owned(),
            root_request_id,
            SymbolIdentifier::with_file_path("New symbol incoming...!", &fs_file_path), // this is for ui event - consider what to pass for symbol_name
            uuid::Uuid::new_v4().to_string(),
            self.message_properties.ui_sender().clone(),
            None,
            self.message_properties.editor_url().clone(),
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

    #[error("Tool Error: {0}")]
    SymbolError(#[from] SymbolError),

    #[error("Wrong tool output")]
    WrongToolOutput(),

    #[error("Step not found: {0}")]
    StepNotFound(usize),

    #[error("Invalid step execution request: {0}")]
    InvalidStepExecution(usize),
}
