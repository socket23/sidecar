use axum::{response::IntoResponse, Extension, Json};

use crate::{application::application::Application, chunking::text_document::Range};

use super::{
    in_line_agent::TextDocumentWeb,
    types::{ApiResponse, Result},
};

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
    Ok(Json(ExtractDocumentationStringResponse {
        documentation: documentation_strings,
    }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractDiagnosticsRangeQuery {
    range: Range,
    text_document_web: TextDocumentWeb,
    threshold_to_expand: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractDiagnosticRangeReply {
    range: Range,
}

pub async fn extract_diagnostics_range(
    Extension(app): Extension<Application>,
    Json(ExtractDiagnosticsRangeQuery {
        range,
        text_document_web,
        threshold_to_expand,
    }): Json<ExtractDiagnosticsRangeQuery>,
) -> Result<impl IntoResponse> {
    let language_parsing = app.language_parsing.clone();
    let expanded_range = language_parsing.get_fix_range(
        &text_document_web.text,
        &text_document_web.language,
        &range,
        threshold_to_expand,
    );
    Ok(Json(ExtractDiagnosticRangeReply {
        range: expanded_range.unwrap_or(range),
    }))
}
