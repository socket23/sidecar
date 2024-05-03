use llm_client::clients::types::LLMClientError;
use thiserror::Error;

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
}
