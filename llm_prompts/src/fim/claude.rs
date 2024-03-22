use super::types::FillInMiddleFormatter;
use either::Either;
use llm_client::clients::types::{LLMClientCompletionRequest, LLMClientCompletionStringRequest};

pub struct ClaudeFillInMiddleFormatter;

impl ClaudeFillInMiddleFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl FillInMiddleFormatter for ClaudeFillInMiddleFormatter {
    fn fill_in_middle(
        &self,
        request: super::types::FillInMiddleRequest,
    ) -> Either<LLMClientCompletionRequest, LLMClientCompletionStringRequest> {
        unimplemented!();
    }
}
