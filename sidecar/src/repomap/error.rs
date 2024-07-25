use core::fmt;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoMapError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Symbol analysis error: {0}")]
    SymbolAnalysisError(String),

    #[error("Graph analysis error: {0}")]
    GraphAnalysisError(String),
}
