use std::sync::Arc;

use llm_client::{broker::LLMBroker, clients::types::LLMType, tokenizer::tokenizer::LLMTokenizer};
use llm_prompts::answer_model::LLMAnswerModelBroker;

use crate::{
    inline_completion::helpers::fix_vscode_position,
    webserver::{
        inline_completion::{InlineCompletionRequest, InlineCompletionResponse},
        model_selection::LLMClientConfig,
    },
};

pub struct InlineCompletionAgent {
    llm_broker: Arc<LLMBroker>,
    llm_tokenizer: Arc<LLMTokenizer>,
    answer_mode: LLMAnswerModelBroker,
}

#[derive(thiserror::Error, Debug)]
pub enum InLineCompletionError {
    #[error("LLM type {0} is not supported for inline completion.")]
    LLMNotSupported(LLMType),
}

struct InLineCompletionData {
    prefix: String,
    suffix: String,
    line: String,
}

impl InlineCompletionAgent {
    pub fn new(
        llm_broker: Arc<LLMBroker>,
        llm_tokenizer: Arc<LLMTokenizer>,
        answer_mode: LLMAnswerModelBroker,
    ) -> Self {
        Self {
            llm_broker,
            llm_tokenizer,
            answer_mode,
        }
    }

    pub async fn completion(
        &self,
        completion_request: InlineCompletionRequest,
        model_config: LLMClientConfig,
    ) -> Result<InlineCompletionResponse, InLineCompletionError> {
        // Now that we have the position, we want to create the request for the fill
        // in the middle request.
        let fast_model = model_config.fast_model.clone();
        let model_config = self.answer_mode.get_answer_model(&fast_model);
        if let None = model_config {
            return Err(InLineCompletionError::LLMNotSupported(fast_model));
        }
        unimplemented!();
    }

    fn generate_prefix_and_suffix(
        completion_request: InlineCompletionRequest,
        answer_mode: &LLMAnswerModelBroker,
    ) -> InLineCompletionData {
        let text_bytes = completion_request.text.as_bytes();
        let position = fix_vscode_position(completion_request.position, text_bytes);
        let path = format!("Path: {}", completion_request.filepath);
        unimplemented!();
    }
}
