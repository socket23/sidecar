use std::collections::HashMap;

use llm_client::clients::types::LLMType;

use super::{codellama::CodeLlamaFillInMiddleFormatter, deepseek::DeepSeekFillInMiddleFormatter};

#[derive(thiserror::Error, Debug)]
pub enum FillInMiddleError {
    #[error("Unknown LLM type")]
    UnknownLLMType,
}

pub struct FillInMiddleRequest {
    prefix: String,
    suffix: String,
}

impl FillInMiddleRequest {
    pub fn new(prefix: String, suffix: String) -> Self {
        Self { prefix, suffix }
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn suffix(&self) -> &str {
        &self.suffix
    }
}

pub struct FillInMiddleResponse {
    pub filled: String,
}

impl FillInMiddleResponse {
    pub fn new(filled: String) -> Self {
        Self { filled }
    }
}

pub trait FillInMiddleFormatter {
    fn fill_in_middle(&self, request: FillInMiddleRequest) -> FillInMiddleResponse;
}

pub struct FillInMiddleBroker {
    providers: HashMap<LLMType, Box<dyn FillInMiddleFormatter + Send + Sync>>,
}

impl FillInMiddleBroker {
    pub fn new() -> Self {
        let broker = Self {
            providers: HashMap::new(),
        };
        broker
            .add_llm(
                LLMType::CodeLlama13BInstruct,
                Box::new(CodeLlamaFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder1_3BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder6BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
    }

    fn add_llm(
        mut self,
        llm_type: LLMType,
        formatter: Box<dyn FillInMiddleFormatter + Send + Sync>,
    ) -> Self {
        self.providers.insert(llm_type, formatter);
        self
    }

    pub fn format_context(
        &self,
        request: FillInMiddleRequest,
        model: &LLMType,
    ) -> Result<FillInMiddleResponse, FillInMiddleError> {
        let formatter = self
            .providers
            .get(model)
            .ok_or(FillInMiddleError::UnknownLLMType)?;
        Ok(formatter.fill_in_middle(request))
    }
}
