use either::Either;
use std::collections::HashMap;

use llm_client::clients::types::{
    LLMClientCompletionRequest, LLMClientCompletionStringRequest, LLMType,
};

use super::{
    claude::ClaudeFillInMiddleFormatter, codellama::CodeLlamaFillInMiddleFormatter,
    deepseek::DeepSeekFillInMiddleFormatter,
};

#[derive(thiserror::Error, Debug)]
pub enum FillInMiddleError {
    #[error("Unknown LLM type")]
    UnknownLLMType,
}

pub struct FillInMiddleRequest {
    prefix: String,
    suffix: String,
    llm_type: LLMType,
    stop_words: Vec<String>,
    completion_tokens: Option<i64>,
}

impl FillInMiddleRequest {
    pub fn new(
        prefix: String,
        suffix: String,
        llm_type: LLMType,
        stop_words: Vec<String>,
        completion_tokens: Option<i64>,
    ) -> Self {
        Self {
            prefix,
            suffix,
            llm_type,
            stop_words,
            completion_tokens,
        }
    }

    pub fn llm(&self) -> &LLMType {
        &self.llm_type
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    pub fn stop_words(self) -> Vec<String> {
        self.stop_words
    }

    pub fn completion_tokens(&self) -> Option<usize> {
        self.completion_tokens.map(|tokens| tokens as usize)
    }
}

pub trait FillInMiddleFormatter {
    fn fill_in_middle(
        &self,
        request: FillInMiddleRequest,
    ) -> Either<LLMClientCompletionRequest, LLMClientCompletionStringRequest>;
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
                LLMType::CodeLlama7BInstruct,
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
            .add_llm(
                LLMType::DeepSeekCoder33BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeHaiku,
                Box::new(ClaudeFillInMiddleFormatter::new()),
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
    ) -> Result<
        Either<LLMClientCompletionRequest, LLMClientCompletionStringRequest>,
        FillInMiddleError,
    > {
        let formatter = self
            .providers
            .get(model)
            .ok_or(FillInMiddleError::UnknownLLMType)?;
        Ok(formatter.fill_in_middle(request))
    }
}
