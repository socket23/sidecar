use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::{
    agentic::tool::{
        base::Tool,
        errors::ToolError,
        input::ToolInput,
        lsp::{diagnostics::Diagnostic, quick_fix::QuickFixOption},
        output::ToolOutput,
    },
    chunking::text_document::Range,
};

use super::{models::anthropic::AnthropicCodeSymbolImportant, types::CodeSymbolError};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename = "reply")]
pub struct CodeCorrectnessAction {
    thinking: String,
    index: i64,
}

impl CodeCorrectnessAction {
    pub fn thinking(&self) -> &str {
        &self.thinking
    }

    pub fn index(&self) -> i64 {
        self.index
    }
}

#[derive(Debug, Clone)]
pub struct CodeCorrectnessRequest {
    fs_file_contents: String,
    fs_file_path: String,
    code_above: Option<String>,
    code_below: Option<String>,
    code_in_selection: String,
    symbol_name: String,
    instruction: String,
    previous_code: String,
    diagnostics: Vec<Diagnostic>,
    quick_fix_actions: Vec<QuickFixOption>,
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
}

impl CodeCorrectnessRequest {
    pub fn new(
        fs_file_contents: String,
        fs_file_path: String,
        code_above: Option<String>,
        code_below: Option<String>,
        code_in_selection: String,
        symbol_name: String,
        instruction: String,
        diagnostics: Vec<Diagnostic>,
        quick_fix_actions: Vec<QuickFixOption>,
        previous_code: String,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
    ) -> Self {
        Self {
            fs_file_contents,
            fs_file_path,
            code_above,
            code_below,
            code_in_selection,
            previous_code,
            quick_fix_actions,
            instruction,
            symbol_name,
            diagnostics,
            llm,
            provider,
            api_keys,
        }
    }
    pub fn file_content(&self) -> &str {
        &self.fs_file_contents
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.diagnostics.as_slice()
    }

    pub fn quick_fix_actions(&self) -> &[QuickFixOption] {
        self.quick_fix_actions.as_slice()
    }

    pub fn code_above(&self) -> Option<String> {
        self.code_above.clone()
    }

    pub fn code_below(&self) -> Option<String> {
        self.code_below.clone()
    }

    pub fn code_in_selection(&self) -> &str {
        &self.code_in_selection
    }

    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    pub fn previous_code(&self) -> &str {
        &self.previous_code
    }

    pub fn llm(&self) -> &LLMType {
        &self.llm
    }

    pub fn llm_provider(&self) -> &LLMProvider {
        &self.provider
    }

    pub fn llm_api_keys(&self) -> &LLMProviderAPIKeys {
        &self.api_keys
    }
}

#[async_trait]
pub trait CodeCorrectness {
    async fn decide_tool_use(
        &self,
        code_correctness_request: CodeCorrectnessRequest,
    ) -> Result<CodeCorrectnessAction, CodeSymbolError>;
}

pub struct CodeCorrectnessBroker {
    llms: HashMap<LLMType, Box<dyn CodeCorrectness + Send + Sync>>,
}

impl CodeCorrectnessBroker {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        let mut llms: HashMap<LLMType, Box<dyn CodeCorrectness + Send + Sync>> = Default::default();
        llms.insert(
            LLMType::ClaudeHaiku,
            Box::new(AnthropicCodeSymbolImportant::new(llm_client.clone())),
        );
        llms.insert(
            LLMType::ClaudeSonnet,
            Box::new(AnthropicCodeSymbolImportant::new(llm_client.clone())),
        );
        llms.insert(
            LLMType::ClaudeOpus,
            Box::new(AnthropicCodeSymbolImportant::new(llm_client.clone())),
        );
        llms.insert(
            LLMType::Gpt4O,
            Box::new(AnthropicCodeSymbolImportant::new(llm_client.clone())),
        );
        llms.insert(
            LLMType::GeminiPro,
            Box::new(AnthropicCodeSymbolImportant::new(llm_client.clone())),
        );
        Self { llms }
    }
}

#[async_trait]
impl Tool for CodeCorrectnessBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.code_correctness_action()?;
        if let Some(implementation) = self.llms.get(context.llm()) {
            implementation
                .decide_tool_use(context)
                .await
                .map(|response| ToolOutput::code_correctness_action(response))
                .map_err(|e| ToolError::CodeSymbolError(e))
        } else {
            Err(ToolError::WrongToolInput)
        }
    }
}
