use axum::{response::IntoResponse, Extension, Json};

use crate::application::application::Application;

use super::types::{ApiResponse, Result};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractDocumentationStringRequest {
    language: String,
    source: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ExtractDocumentationStringResponse {
    documentation: Vec<String>,
}

impl ApiResponse for ExtractDocumentationStringResponse {}

pub async fn extract_documentation_strings(
    Extension(app): Extension<Application>,
    Json(ExtractDocumentationStringRequest { language, source }): Json<
        ExtractDocumentationStringRequest,
    >,
) -> Result<impl IntoResponse> {
    let language_parsing = app.language_parsing.clone();
    let documentation_strings = language_parsing.parse_documentation(&source, &language);
    dbg!(documentation_strings.clone());
    Ok(Json(ExtractDocumentationStringResponse {
        documentation: documentation_strings,
    }))
}
