use axum::response::IntoResponse;
use axum::{Extension, Json};

use crate::application::application::Application;

use super::types::Result;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditFileRequest {
    pub file_path: String,
    pub file_content: String,
    pub new_content: String,
}

pub async fn file_edit(
    Extension(app): Extension<Application>,
    Json(EditFileRequest {
        file_path,
        file_content,
        new_content,
    }): Json<EditFileRequest>,
) -> Result<impl IntoResponse> {
    Ok(vec![])
}
