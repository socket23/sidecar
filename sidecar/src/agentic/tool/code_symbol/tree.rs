use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use llm_client::{broker::LLMBroker, clients::types::LLMType};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

use super::{
    important::CodeSymbolImportantResponse, models::anthropic::AnthropicCodeSymbolImportant,
    repo_map_search::RepoMapSearch, types::CodeSymbolError,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportantFilesFinderQuery {
    tree: String,
    llm: LLMType,
}

impl ImportantFilesFinderQuery {
    pub fn new(tree: String, llm: LLMType) -> Self {
        Self { tree, llm }
    }

    pub fn llm(&self) -> &LLMType {
        &self.llm
    }
}

#[async_trait]
pub trait ImportantFilesFinder {
    async fn find_important_files(
        &self,
        request: ImportantFilesFinderQuery,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError>;
}

pub struct ImportantFilesFinderBroker {
    llms: HashMap<LLMType, Box<dyn ImportantFilesFinder + Send + Sync>>,
}

impl ImportantFilesFinderBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        let mut llms: HashMap<LLMType, Box<dyn ImportantFilesFinder + Send + Sync>> =
            Default::default();
        llms.insert(
            LLMType::ClaudeHaiku,
            Box::new(AnthropicCodeSymbolImportant::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        llms.insert(
            LLMType::ClaudeSonnet,
            Box::new(AnthropicCodeSymbolImportant::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        llms.insert(
            LLMType::ClaudeOpus,
            Box::new(AnthropicCodeSymbolImportant::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        llms.insert(
            LLMType::GeminiPro,
            Box::new(AnthropicCodeSymbolImportant::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );
        llms.insert(
            LLMType::GeminiProFlash,
            Box::new(AnthropicCodeSymbolImportant::new(
                llm_client.clone(),
                fail_over_llm,
            )),
        );
        Self { llms }
    }
}

#[async_trait]
impl Tool for ImportantFilesFinderBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = input.important_files_finder_query()?;
        if let Some(implementation) = self.llms.get(request.llm()) {
            let output = implementation
                .find_important_files(request)
                .await
                .map_err(|e| ToolError::CodeSymbolError(e))?;
            Ok(ToolOutput::ImportantFilesFinder(output))
        } else {
            Err(ToolError::LLMNotSupported)
        }
    }
}
