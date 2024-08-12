use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "response")]
pub struct IdentifyResponse {
    #[serde(rename = "item")]
    pub item: Vec<IdentifiedFile>,
    pub scratch_pad: String,
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

    pub fn thinking(&self) -> &str {
        &self.thinking
    }
}
