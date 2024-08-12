use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "response")]
pub struct DecideResponse {
    suggestions: String,
    complete: bool,
}

impl DecideResponse {
    pub fn suggestions(&self) -> &str {
        &self.suggestions
    }

    pub fn complete(&self) -> bool {
        self.complete
    }
}
