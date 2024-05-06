//! We want to invoke the code edit and rewrite a section of the code which we
//! are insterested in
//! The input here is the file_path and the range to edit and the new_output which
//! we want to generate

use std::sync::Arc;

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::agentic::tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput};

use super::models::broker::CodeEditBroker;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CodeEdit {
    code_above: Option<String>,
    code_below: Option<String>,
    fs_file_path: String,
    code_to_edit: String,
    extra_context: String,
    language: String,
    model: LLMType,
    instruction: String,
    api_key: LLMProviderAPIKeys,
    provider: LLMProvider,
}

pub struct CodeEditingTool {
    llm_client: Arc<LLMBroker>,
    broker: Arc<CodeEditBroker>,
}

impl CodeEditingTool {
    pub fn new(llm_client: Arc<LLMBroker>, broker: Arc<CodeEditBroker>) -> Self {
        Self { llm_client, broker }
    }
}

impl CodeEdit {
    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    pub fn above_context(&self) -> Option<&str> {
        self.code_above
            .as_ref()
            .map(|above_context| above_context.as_str())
    }

    pub fn below_context(&self) -> Option<&str> {
        self.code_below
            .as_ref()
            .map(|below_context| below_context.as_str())
    }

    pub fn code_to_edit(&self) -> &str {
        &self.code_to_edit
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn extra_content(&self) -> &str {
        &self.extra_context
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn model(&self) -> &LLMType {
        &self.model
    }
}

#[async_trait]
impl Tool for CodeEditingTool {
    // TODO(skcd): Figure out how we want to do streaming here in the future
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let code_edit_context = input.is_code_edit()?;
        let llm_message = self.broker.format_prompt(&code_edit_context)?;
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        self.llm_client
            .stream_completion(
                code_edit_context.api_key,
                llm_message,
                code_edit_context.provider,
                vec![("request".to_owned(), "code_edit_tool".to_owned())]
                    .into_iter()
                    .collect(),
                sender,
            )
            .await
            .map(|result| ToolOutput::code_edit_output(result))
            .map_err(|e| ToolError::LLMClientError(e))
    }
}
