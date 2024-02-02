use axum::{response::IntoResponse, Extension, Json};

use crate::{
    application::application::Application,
    chunking::text_document::{Position, Range},
};

use super::types::{ApiResponse, Result};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct InlineCompletionRequest {
    pub filepath: String,
    pub language: String,
    pub text: String,
    pub position: Position,
    pub indentation: Option<String>,
    pub clipboard: Option<String>,
    pub manually: Option<bool>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineCompletion {
    pub insert_text: String,
    pub range: Range,
    pub filter_text: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineCompletionResponse {
    pub completions: Vec<InlineCompletion>,
}

impl ApiResponse for InlineCompletionResponse {}

pub async fn inline_completion(
    Extension(app): Extension<Application>,
    Json(InlineCompletionRequest {
        filepath,
        language,
        text,
        position,
        indentation,
        clipboard,
        manually,
    }): Json<InlineCompletionRequest>,
) -> Result<impl IntoResponse> {
    Ok(Json(InlineCompletionResponse {
        completions: vec![],
    }))
}
