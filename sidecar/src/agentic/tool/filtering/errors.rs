use llm_client::clients::types::LLMClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodeToEditFilteringError {
    #[error("LLM Client error: {0}")]
    LLMClientError(LLMClientError),
}
