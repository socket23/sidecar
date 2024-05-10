use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::agentic::{
    symbol::identifier::Snippet,
    tool::{
        base::Tool, errors::ToolError, filtering::errors::CodeToEditFilteringError,
        input::ToolInput, output::ToolOutput,
    },
};

use super::models::anthropic::AnthropicCodeToEditFormatter;

#[derive(Debug, Clone)]
pub struct SnippetWithReason {
    snippet: Snippet,
    reason: String,
}

impl SnippetWithReason {
    pub fn new(snippet: Snippet, reason: String) -> Self {
        Self { snippet, reason }
    }
}

#[derive(Debug, Clone)]
pub struct CodeToEditFilterResponse {
    snippets_to_edit_ordered: Vec<SnippetWithReason>,
    snippets_to_not_edit: Vec<SnippetWithReason>,
}

impl CodeToEditFilterResponse {
    pub fn new(
        snippets_to_edit: Vec<SnippetWithReason>,
        snippets_to_not_edit: Vec<SnippetWithReason>,
    ) -> Self {
        Self {
            snippets_to_edit_ordered: snippets_to_edit,
            snippets_to_not_edit: snippets_to_not_edit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodeToEditFilterRequest {
    snippets: Vec<Snippet>,
    query: String,
    llm: LLMType,
    provider: LLMProvider,
    api_key: LLMProviderAPIKeys,
}

impl CodeToEditFilterRequest {
    pub fn new(
        snippets: Vec<Snippet>,
        query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_key: LLMProviderAPIKeys,
    ) -> Self {
        Self {
            snippets,
            query,
            llm,
            provider,
            api_key,
        }
    }

    pub fn get_snippets(&self) -> &[Snippet] {
        &self.snippets
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn llm(&self) -> &LLMType {
        &self.llm
    }

    pub fn provider(&self) -> &LLMProvider {
        &self.provider
    }

    pub fn api_key(&self) -> &LLMProviderAPIKeys {
        &self.api_key
    }
}

pub struct CodeToEditFormatterBroker {
    pub llms: HashMap<LLMType, Box<dyn CodeToEditFilterFormatter + Send + Sync>>,
}

impl CodeToEditFormatterBroker {
    pub fn new(llm_broker: Arc<LLMBroker>) -> Self {
        let mut llms: HashMap<LLMType, Box<dyn CodeToEditFilterFormatter + Send + Sync>> =
            Default::default();
        llms.insert(
            LLMType::ClaudeHaiku,
            Box::new(AnthropicCodeToEditFormatter::new(llm_broker.clone())),
        );
        llms.insert(
            LLMType::ClaudeSonnet,
            Box::new(AnthropicCodeToEditFormatter::new(llm_broker.clone())),
        );
        llms.insert(
            LLMType::ClaudeOpus,
            Box::new(AnthropicCodeToEditFormatter::new(llm_broker)),
        );
        Self { llms }
    }
}

#[async_trait]
pub trait CodeToEditFilterFormatter {
    async fn filter_code_snippets(
        &self,
        request: CodeToEditFilterRequest,
    ) -> Result<CodeToEditFilterResponse, CodeToEditFilteringError>;

    // async fn filter_code_snippets_inside_symbol(
    //     &self,
    //     ranked_snippets_single_symbol: String,
    // ) -> Result<CodeToEditFilterResponse, CodeToEditFilteringError>;
}

#[async_trait]
impl Tool for CodeToEditFormatterBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.filter_code_snippets_for_editing()?;
        if let Some(llm) = self.llms.get(&context.llm) {
            llm.filter_code_snippets(context)
                .await
                .map_err(|e| ToolError::CodeToEditFiltering(e))
                .map(|result| ToolOutput::CodeToEditSnippets(result))
        } else {
            Err(ToolError::WrongToolInput)
        }
    }
}
