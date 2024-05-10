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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_edit")]
pub struct CodeToEditSnippet {
    id: usize,
    reason_to_edit: String,
}

impl CodeToEditSnippet {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn reason_to_edit(&self) -> &str {
        &self.reason_to_edit
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_not_edit")]
pub struct CodeToNotEditSnippet {
    id: usize,
    reason_to_not_edit: String,
}

impl CodeToNotEditSnippet {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn reason_to_not_edit(&self) -> &str {
        &self.reason_to_not_edit
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_edit_list")]
pub struct CodeToEditList {
    #[serde(rename = "$value")]
    snippets: Vec<CodeToEditSnippet>,
}

impl CodeToEditList {
    pub fn snippets(&self) -> &[CodeToEditSnippet] {
        self.snippets.as_slice()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_not_edit_list")]
pub struct CodeToNotEditList {
    #[serde(rename = "$value")]
    snippets: Vec<CodeToNotEditSnippet>,
}

impl CodeToNotEditList {
    pub fn snippets(&self) -> &[CodeToNotEditSnippet] {
        self.snippets.as_slice()
    }
}

#[derive(Debug, Clone)]
pub struct CodeToEditSymbolResponse {
    code_to_edit_list: CodeToEditList,
    code_to_not_edit_list: CodeToNotEditList,
}

impl CodeToEditSymbolResponse {
    pub fn new(
        code_to_edit_list: CodeToEditList,
        code_to_not_edit_list: CodeToNotEditList,
    ) -> Self {
        Self {
            code_to_edit_list,
            code_to_not_edit_list,
        }
    }

    pub fn code_to_edit_list(&self) -> &CodeToEditList {
        &self.code_to_edit_list
    }

    pub fn code_to_not_edit_list(&self) -> &CodeToNotEditList {
        &self.code_to_not_edit_list
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

#[derive(Debug, Clone)]
pub struct CodeToEditSymbolRequest {
    xml_symbol: String,
    query: String,
    llm: LLMType,
    provider: LLMProvider,
    api_key: LLMProviderAPIKeys,
}

impl CodeToEditSymbolRequest {
    pub fn new(
        xml_symbol: String,
        query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_key: LLMProviderAPIKeys,
    ) -> Self {
        Self {
            xml_symbol,
            query,
            llm,
            api_key,
            provider,
        }
    }

    pub fn xml_string(self) -> String {
        self.xml_symbol
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

    // TODO(skcd): We need to figure out which symbols we need to keep
    async fn filter_code_snippets_inside_symbol(
        &self,
        request: CodeToEditSymbolRequest,
    ) -> Result<CodeToEditSymbolResponse, CodeToEditFilteringError>;
}

#[async_trait]
impl Tool for CodeToEditFormatterBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.filter_code_snippets_request()?;
        match context {
            either::Left(request) => {
                if let Some(llm) = self.llms.get(&request.llm) {
                    llm.filter_code_snippets(request)
                        .await
                        .map_err(|e| ToolError::CodeToEditFiltering(e))
                        .map(|result| ToolOutput::CodeToEditSnippets(result))
                } else {
                    Err(ToolError::WrongToolInput)
                }
            }
            either::Right(context) => {
                if let Some(llm) = self.llms.get(&context.llm) {
                    llm.filter_code_snippets_inside_symbol(context)
                        .await
                        .map_err(|e| ToolError::CodeToEditFiltering(e))
                        .map(|result| ToolOutput::CodeToEditSingleSymbolSnippets(result))
                } else {
                    Err(ToolError::WrongToolInput)
                }
            }
        }
    }
}
