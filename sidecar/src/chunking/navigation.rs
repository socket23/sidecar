use std::ops::Range as OpsRange;

use super::{scope_graph::Symbol, text_document::Range};

#[derive(Debug, serde::Serialize)]
pub struct FileSymbols {
    /// The file to which the following occurrences belong
    pub file: String,

    /// A collection of symbol locations with context in this file
    pub data: Vec<Occurrence>,
}

#[derive(serde::Serialize, Debug)]
pub struct Occurrence {
    pub kind: OccurrenceKind,
    pub range: Range,
    pub snippet: Snippet,
}

#[derive(serde::Serialize, Debug)]
pub struct Snippet {
    pub data: String,
    pub line_range: OpsRange<usize>,
    pub symbols: Vec<Symbol>,
}

impl Occurrence {
    pub fn is_definition(&self) -> bool {
        matches!(self.kind, OccurrenceKind::Definition)
    }
}

#[derive(serde::Serialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum OccurrenceKind {
    #[default]
    Reference,
    Definition,
}
