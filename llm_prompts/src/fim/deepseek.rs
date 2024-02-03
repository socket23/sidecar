use super::types::{FillInMiddleFormatter, FillInMiddleRequest, FillInMiddleResponse};

pub struct DeepSeekFillInMiddleFormatter;

impl DeepSeekFillInMiddleFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl FillInMiddleFormatter for DeepSeekFillInMiddleFormatter {
    fn fill_in_middle(&self, request: FillInMiddleRequest) -> FillInMiddleResponse {
        // format is
        // <｜fim▁begin｜>{{{prefix}}}<｜fim▁hole｜>{{{suffix}}}<｜fim▁end｜>
        // https://ollama.ai/library/deepseek
        let prefix = request.prefix();
        let suffix = request.suffix();
        let response = format!("<｜fim▁begin｜>{prefix}<｜fim▁hole｜>{suffix}<｜fim▁end｜>");
        FillInMiddleResponse::new(response)
    }
}
