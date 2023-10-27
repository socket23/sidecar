use axum::{response::IntoResponse, Json};

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
    Json(ExtractDocumentationStringRequest { language, source }): Json<
        ExtractDocumentationStringRequest,
    >,
) -> Result<impl IntoResponse> {
    match language.as_ref() {
        "typescript" | "typescriptreact" => {
            let documentation =
                crate::chunking::documentation_parsing::parse_documentation_for_typescript_code(
                    &source,
                );
            Ok(Json(ExtractDocumentationStringResponse { documentation }))
        }
        // If we have no matching, just return an empty list here, we will show
        // the diff for the whole chunk
        _ => Ok(Json(ExtractDocumentationStringResponse {
            documentation: Vec::new(),
        })),
    }
}
