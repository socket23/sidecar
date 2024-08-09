use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "responses")]
pub struct IdentifyResponse {
    #[serde(rename = "response")]
    pub responses: Vec<IdentifiedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedFile {
    path: PathBuf,
    thinking: String,
}

impl IdentifiedFile {
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
