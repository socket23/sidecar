use std::collections::HashMap;

use llm_client::clients::types::{LLMClientCompletionRequest, LLMType};

use crate::agentic::tool::{code_edit::types::CodeEdit, errors::ToolError};

use super::anthropic::AnthropicCodeEditFromatter;

pub trait CodeEditPromptFormatters {
    fn format_prompt(&self, context: &CodeEdit) -> LLMClientCompletionRequest;
}

pub struct CodeEditBroker {
    models: HashMap<LLMType, Box<dyn CodeEditPromptFormatters + Send + Sync>>,
}

impl CodeEditBroker {
    pub fn new() -> Self {
        let mut models: HashMap<LLMType, Box<dyn CodeEditPromptFormatters + Send + Sync>> =
            HashMap::new();
        models.insert(
            LLMType::ClaudeHaiku,
            Box::new(AnthropicCodeEditFromatter::new()),
        );
        models.insert(
            LLMType::ClaudeSonnet,
            Box::new(AnthropicCodeEditFromatter::new()),
        );
        models.insert(
            LLMType::ClaudeOpus,
            Box::new(AnthropicCodeEditFromatter::new()),
        );
        Self { models }
    }

    pub fn format_prompt(
        &self,
        context: &CodeEdit,
    ) -> Result<LLMClientCompletionRequest, ToolError> {
        let model = context.model();
        if let Some(formatter) = self.models.get(model) {
            Ok(formatter.format_prompt(context))
        } else {
            Err(ToolError::LLMNotSupported)
        }
    }
}
