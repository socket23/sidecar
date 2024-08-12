use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "response")]
pub struct DecideResponse {
    suggestions: String,
    complete: bool,
}
