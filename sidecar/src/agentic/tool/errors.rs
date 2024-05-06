use llm_client::clients::types::LLMClientError;
use thiserror::Error;

use super::{code_symbol::types::CodeSymbolError, rerank::base::ReRankError};

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Unable to grab the context")]
    UnableToGrabContext,

    #[error("LLM not supported for tool")]
    LLMNotSupported,

    #[error("Wrong tool input found")]
    WrongToolInput,

    #[error("LLM Client call error: {0}")]
    LLMClientError(LLMClientError),

    #[error("Missing tool")]
    MissingTool,

    #[error("Error converting serde json to string")]
    SerdeConversionFailed,

    #[error("Communication with editor failed")]
    ErrorCommunicatingWithEditor,

    #[error("Language not supported")]
    NotSupportedLanguage,

    #[error("ReRanking error: {0}")]
    ReRankingError(ReRankError),

    #[error("Code Symbol Error: {0}")]
    CodeSymbolError(CodeSymbolError),

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
}
