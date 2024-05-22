use thiserror::Error;
use tokio::sync::oneshot::error::RecvError;

use crate::agentic::tool::errors::ToolError;

#[derive(Debug, Error)]
pub enum SymbolError {
    #[error("Tool error: {0}")]
    ToolError(ToolError),

    #[error("Wrong tool output")]
    WrongToolOutput,

    #[error("Expected file to exist")]
    ExpectedFileToExist,

    #[error("Symbol not found")]
    SymbolNotFound,

    #[error("Unable to read file contents")]
    UnableToReadFileContent,

    #[error("channel recieve error: {0}")]
    RecvError(RecvError),

    #[error("No definition found: {0}")]
    DefinitionNotFound(String),

    #[error("Symbol not contained in a child")]
    SymbolNotContainedInChild,

    #[error("No containing symbol found")]
    NoContainingSymbolFound,
}
