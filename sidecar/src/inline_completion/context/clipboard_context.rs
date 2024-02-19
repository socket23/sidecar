use std::sync::Arc;

use llm_client::{clients::types::LLMType, tokenizer::tokenizer::LLMTokenizer};

use crate::chunking::editor_parsing::EditorParsing;

pub struct ClipboardContext {
    clipboard_context: String,
    tokenizer: Arc<LLMTokenizer>,
    llm_type: LLMType,
    editor_parsing: Arc<EditorParsing>,
}
