use std::sync::Arc;

use futures::{stream, StreamExt};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

use crate::{
    agentic::{
        symbol::{
            errors::SymbolError,
            events::message_event::SymbolEventMessageProperties,
            identifier::{LLMProperties, SymbolIdentifier},
            tool_box::ToolBox,
        },
        tool::{code_edit::search_and_replace::SearchAndReplaceEditingRequest, errors::ToolError},
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
    tool_box: Arc<ToolBox>,
    llm_properties: LLMProperties,
}

impl PlanService {
    pub fn new(tool_box: Arc<ToolBox>, llm_properties: LLMProperties) -> Self {
        Self {
            tool_box,
            llm_properties,
        }
    }

    pub async fn save_plan(&self, plan: &Plan, path: &str) -> std::io::Result<()> {
        let serialized = serde_json::to_string(plan).unwrap();
        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(serialized.as_bytes()).await?;
        Ok(())
    }

    pub async fn load_plan(&self, path: &str) -> std::io::Result<Plan> {
        let content = tokio::fs::read_to_string(path).await?;
        let plan: Plan = serde_json::from_str(&content).unwrap();
        Ok(plan)
    }

    pub async fn create_plan(
        &self,
        plan_id: String,
        query: String,
        user_context: UserContext,
        plan_storage_path: String,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Plan, ServiceError> {
        let plan_steps = self
            .tool_box
            .generate_plan(&query, &user_context, message_properties)
            .await?;

        Ok(Plan::new(
            plan_id.to_owned(),
            "Placeholder Title (to be computed)".to_owned(),
            "".to_owned(),
            query,
            plan_steps,
            plan_storage_path,
        )
        .with_user_context(user_context))
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

    pub async fn prepare_context(&self, steps: &[PlanStep], checkpoint: usize) -> String {
        let contexts = self.step_execution_context(steps, checkpoint);
        // todo(zi) consider accumulating this in a context manager vs recomputing for each step (long)
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

        full_context_as_string
    }

    pub async fn execute_step(
        &self,
        step: &PlanStep,
        context: String,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<(), ServiceError> {
        let instruction = step.description();
        let fs_file_path = step.file_to_edit();

        let file_content = self
            .tool_box
            .file_open(fs_file_path.clone(), message_properties.clone())
            .await?
            .contents();
        let request = SearchAndReplaceEditingRequest::new(
            fs_file_path.to_owned(),
            Range::default(),
            file_content.to_owned(), // this is needed too?
            file_content.to_owned(),
            context, // todo(zi): consider giving system_prompt more info about this being plan history
            self.llm_properties.clone(),
            None,
            instruction.to_owned(),
            message_properties.root_request_id().to_owned(),
            SymbolIdentifier::with_file_path("New symbol incoming...!", &fs_file_path), // this is for ui event - consider what to pass for symbol_name
            uuid::Uuid::new_v4().to_string(),
            message_properties.ui_sender().clone(),
            None,
            message_properties.editor_url().clone(),
            None,
            vec![],
            vec![],
            false,
        );

        let _ = self
            .tool_box
            .execute_search_and_replace_edit(request)
            .await?;

        // todo(zi): surprisingly, there's not much to do after edit.

        Ok(())
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
