use super::exp::SearchResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "responses")]
pub struct IdentifyResponse {
    #[serde(rename = "response")]
    pub responses: Vec<SearchResult>,
}
