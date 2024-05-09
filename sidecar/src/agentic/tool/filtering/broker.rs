use async_trait::async_trait;
use std::collections::HashMap;

use llm_client::{
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

#[derive(Debug, Clone)]
pub struct CodeToEditFilterResponse {
    snippets: Vec<Snippet>,
}

pub struct CodeToEditFormatterBroker {
    pub llms: HashMap<LLMType, Box<dyn CodeToEditFilterFormatter + Send + Sync>>,
}

#[async_trait]
pub trait CodeToEditFilterFormatter {
    async fn filter_code_snippets(
        &self,
        request: CodeToEditFilterRequest,
    ) -> Result<Vec<Snippet>, CodeToEditFilteringError>;
}

#[async_trait]
impl Tool for CodeToEditFormatterBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.filter_code_snippets_for_editing()?;
        if let Some(llm) = self.llms.get(&context.llm) {
            let response = llm
                .filter_code_snippets(context)
                .await
                .map_err(|e| ToolError::CodeToEditFiltering(e));
            todo!();
        } else {
            Err(ToolError::WrongToolInput)
        }
    }
}
