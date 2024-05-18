use thiserror::Error;

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
}
