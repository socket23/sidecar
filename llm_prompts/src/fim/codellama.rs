use super::types::{FillInMiddleFormatter, FillInMiddleRequest, FillInMiddleResponse};

pub struct CodeLlamaFillInMiddleFormatter;

impl CodeLlamaFillInMiddleFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl FillInMiddleFormatter for CodeLlamaFillInMiddleFormatter {
    fn fill_in_middle(&self, request: FillInMiddleRequest) -> FillInMiddleResponse {
        // format is
        // <PRE> {prefix} <SUF>{suffix} <MID>
        // https://ollama.ai/library/codellama
        let prefix = request.prefix();
        let suffix = request.suffix();
        let response = format!("<PRE> {prefix} <SUF>{suffix} <MID>");
        FillInMiddleResponse::new(response)
    }
}
