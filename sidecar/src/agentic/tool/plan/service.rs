use std::{collections::HashMap, sync::Arc};

use futures::{stream, StreamExt};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

use crate::{
    agentic::{
        symbol::{
            errors::SymbolError,
            events::{
                edit::SymbolToEdit,
                lsp::LSPDiagnosticError,
                message_event::{SymbolEventMessage, SymbolEventMessageProperties},
            },
            identifier::SymbolIdentifier,
            manager::SymbolManager,
            tool_box::ToolBox,
            tool_properties::ToolProperties,
            types::SymbolEventRequest,
        },
        tool::{errors::ToolError, lsp::file_diagnostics::DiagnosticMap},
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
    symbol_manager: Arc<SymbolManager>,
}

impl PlanService {
    pub fn new(tool_box: Arc<ToolBox>, symbol_manager: Arc<SymbolManager>) -> Self {
        Self {
            tool_box,
            symbol_manager,
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

    /// also brings in associated files (requires go to reference)
    async fn process_diagnostics(
        &self,
        files_until_checkpoint: Vec<String>,
        message_properties: SymbolEventMessageProperties,
    ) -> Vec<LSPDiagnosticError> {
        let file_lsp_diagnostics = self
            .tool_box()
            .get_lsp_diagnostics_for_files(files_until_checkpoint, message_properties.clone())
            .await
            .unwrap_or_default();

        // Do not expand over the lsp diagnostics yet
        file_lsp_diagnostics
        // stream::iter(file_lsp_diagnostics)
        //     .map(|mut diagnostic| async {
        //         if let Ok(response) = self
        //             .tool_box
        //             .go_to_references(
        //                 diagnostic.fs_file_path().to_owned(),
        //                 diagnostic.range().start_position().clone(),
        //                 message_properties.clone(),
        //             )
        //             .await
        //         {
        //             let associated_files: Vec<String> = response
        //                 .locations()
        //                 .into_iter()
        //                 .map(|location| location.fs_file_path().to_owned())
        //                 .collect();
        //             diagnostic.set_associated_files(associated_files);
        //         }
        //         diagnostic
        //     })
        //     .buffer_unordered(10) // will this kill editor? yep 💀
        //     .collect::<Vec<_>>()
        //     .await
    }

    /// Appends the step to the point after the checkpoint - diagnostics are included by default
    pub async fn append_steps(
        &self,
        mut plan: Plan,
        query: String,
        user_context: UserContext,
        message_properties: SymbolEventMessageProperties,
        is_deep_reasoning: bool,
    ) -> Result<Plan, PlanServiceError> {
        let plan_checkpoint = plan.checkpoint();
        if let Some(checkpoint) = plan_checkpoint {
            // append to post checkpoint
            // - gather the plan until the checkpoint
            // - gather the git-diff we have until now
            // - the files which we are present we keep that in the context
            // - figure out the new steps which we want and insert them
            let plan_until_now = plan.plan_until_point(checkpoint);
            let files_until_checkpoint = plan.files_in_plan(checkpoint);
            let recent_edits = self
                .tool_box
                .recently_edited_files(
                    files_until_checkpoint.clone().into_iter().collect(),
                    message_properties.clone(),
                )
                .await?;

            // get all diagnostics present on these files
            let file_lsp_diagnostics = self
                .process_diagnostics(files_until_checkpoint, message_properties.clone())
                .await;

            let diagnostics_grouped_by_file: DiagnosticMap =
                file_lsp_diagnostics
                    .into_iter()
                    .fold(HashMap::new(), |mut acc, error| {
                        acc.entry(error.fs_file_path().to_owned())
                            .or_insert_with(Vec::new)
                            .push(error);
                        acc
                    });

            let formatted_diagnostics = Self::format_diagnostics(&diagnostics_grouped_by_file);

            let new_steps = self
                .tool_box
                .generate_new_steps_for_plan(
                    plan_until_now,
                    plan.initial_user_query().to_owned(),
                    query,
                    user_context,
                    recent_edits,
                    message_properties,
                    is_deep_reasoning,
                    formatted_diagnostics,
                )
                .await?;
            plan.add_steps_vec(new_steps);
            let _ = self.save_plan(&plan, plan.storage_path()).await;
            // we want to get the new plan over here and insert it properly
        } else {
            // pushes the steps at the start of the plan
        }
        Ok(plan)
    }

    pub fn format_diagnostics(diagnostics: &DiagnosticMap) -> String {
        diagnostics
            .iter()
            .map(|(file, errors)| {
                let formatted_errors = errors
                    .iter()
                    .map(|error| {
                        format!(
                            "Snippet: {}\nDiagnostic: {}\nFiles Affected: {}",
                            error.snippet(),
                            error.diagnostic_message(),
                            error.associated_files().map_or_else(
                                || String::from("Only this file."),
                                |files| files.join(", ")
                            ),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");

                format!("File: {}\n{}", file, formatted_errors)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub async fn create_plan(
        &self,
        plan_id: String,
        query: String,
        user_context: UserContext,
        is_deep_reasoning: bool,
        plan_storage_path: String,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Plan, PlanServiceError> {
        let plan_steps = self
            .tool_box
            .generate_plan(&query, &user_context, is_deep_reasoning, message_properties)
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

    /// gets all files_to_edit from PlanSteps up to index
    pub fn get_edited_files(&self, plan: &Plan, index: usize) -> Vec<String> {
        plan.steps()[..index]
            .iter()
            .filter_map(|step| step.file_to_edit())
            .collect::<Vec<_>>()
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
        let full_context_as_string = stream::iter(contexts.to_vec().into_iter().enumerate().map(
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

    pub fn tool_box(&self) -> &ToolBox {
        &self.tool_box
    }

    pub async fn execute_step(
        &self,
        step: &PlanStep,
        context: String,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<(), PlanServiceError> {
        let instruction = step.description();
        let fs_file_path = match step.file_to_edit() {
            Some(path) => path,
            None => {
                return Err(PlanServiceError::AbsentFilePath(
                    "No file path provided for editing".to_string(),
                ))
            }
        };

        let hub_sender = self.symbol_manager.hub_sender();
        let (ui_sender, _ui_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (edit_done_sender, edit_done_receiver) = tokio::sync::oneshot::channel();
        let _ = hub_sender.send(SymbolEventMessage::new(
            SymbolEventRequest::simple_edit_request(
                SymbolIdentifier::with_file_path(&fs_file_path, &fs_file_path),
                SymbolToEdit::new(
                    fs_file_path.to_owned(),
                    Range::default(),
                    fs_file_path.to_owned(),
                    vec![instruction.to_owned()],
                    false,
                    false,
                    true,
                    instruction.to_owned(),
                    None,
                    false,
                    Some(context),
                    true,
                    None,
                    vec![],
                ),
                ToolProperties::new(),
            ),
            message_properties.request_id().clone(),
            ui_sender,
            edit_done_sender,
            tokio_util::sync::CancellationToken::new(),
            message_properties.editor_url(),
        ));

        // await on the edit to finish happening
        let _ = edit_done_receiver.await;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum PlanServiceError {
    #[error("Tool Error: {0}")]
    ToolError(#[from] ToolError),

    #[error("Tool Error: {0}")]
    SymbolError(#[from] SymbolError),

    #[error("Wrong tool output")]
    WrongToolOutput(),

    #[error("Step not found: {0}")]
    StepNotFound(usize),

    #[error("Absent file path: {0}")]
    AbsentFilePath(String),

    #[error("Invalid step execution request: {0}")]
    InvalidStepExecution(usize),
}
