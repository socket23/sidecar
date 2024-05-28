use llm_client::clients::types::{LLMClientError, LLMType};
use thiserror::Error;

use crate::user_context::types::UserContextError;

#[derive(Debug, Error)]
pub enum CodeSymbolError {
    #[error("Wrong LLM for input: {0}")]
    WrongLLM(LLMType),

    #[error("LLM Client erorr: {0}")]
    LLMClientError(#[from] LLMClientError),

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_xml_rs::Error),

    #[error("Quick xml error: {0}")]
    QuickXMLError(#[from] quick_xml::DeError),

    #[error("User context error: {0}")]
    UserContextError(#[from] UserContextError),
}
