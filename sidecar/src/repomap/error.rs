use core::fmt;
use std::io;

#[derive(Debug)]
pub enum RepoMapError {
    IoError(io::Error),
    ParseError(String),
    SymbolAnalysisError(String),
    GraphAnalysisError(String),
}

impl fmt::Display for RepoMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoMapError::IoError(e) => write!(f, "I/O error: {}", e),
            RepoMapError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            RepoMapError::SymbolAnalysisError(msg) => write!(f, "Symbol analysis error: {}", msg),
            RepoMapError::GraphAnalysisError(msg) => write!(f, "Graph analysis error: {}", msg),
        }
    }
}

impl From<std::io::Error> for RepoMapError {
    fn from(error: std::io::Error) -> Self {
        RepoMapError::IoError(error)
    }
}
