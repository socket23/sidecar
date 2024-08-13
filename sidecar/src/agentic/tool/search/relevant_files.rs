use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// let the world know that I do not love serde
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "response")]
pub struct QueryRelevantFilesResponse {
    pub files: QueryRelevantFiles,
    pub scratch_pad: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRelevantFiles {
    file: Vec<QueryRelevantFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRelevantFile {
    path: PathBuf,
    thinking: String,
}
