use axum::{response::IntoResponse, Extension, Json};

use crate::{
    application::application::Application,
    chunking::{
        navigation::FileSymbols,
        refdef::{refdef, refdef_runtime},
        text_document::Range,
    },
    repo::types::RepoRef,
};

use super::{
    in_line_agent::{fix_snippet_information, SnippetInformation, TextDocumentWeb},
    types::{ApiResponse, Result},
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TokenInformationRequest {
    pub repo_ref: RepoRef,
    pub relative_path: String,
    pub hovered_text: String,
    pub snippet_information: SnippetInformation,
    pub text_document_web: TextDocumentWeb,
}

#[derive(Debug, serde::Serialize)]
pub struct TokenInformationResponse {
    pub data: Vec<FileSymbols>,
}

impl ApiResponse for TokenInformationResponse {}

pub async fn token_information(
    Extension(app): Extension<Application>,
    Json(TokenInformationRequest {
        repo_ref,
        relative_path,
        hovered_text,
        snippet_information,
        text_document_web,
    }): Json<TokenInformationRequest>,
) -> Result<impl IntoResponse> {
    // let source_doc = app
    //     .indexes
    //     .file
    //     .get_by_path_content_document(&relative_path, &repo_ref)
    //     .await
    //     .map_err(|e| anyhow::anyhow!(e))?
    //     .ok_or_else(|| anyhow::anyhow!("No file found with the file path"))?;
    // let all_docs = app.indexes.file.by_repo(&repo_ref).await;
    // let _ = all_docs
    //     .iter()
    //     .position(|doc| doc.relative_path == relative_path)
    //     .ok_or(anyhow::anyhow!(
    //         "Failed to find source file when getting info by repo"
    //     ));

    let snippet_information =
        fix_snippet_information(snippet_information, text_document_web.utf8_array.as_slice());

    // Lets try to get results from a single file for now and see how we can scale
    // this later for multiple files
    Ok(refdef_runtime(
        &repo_ref,
        &hovered_text,
        &snippet_information.to_range(),
        &relative_path,
        text_document_web.utf8_array.as_slice(),
        text_document_web.language.as_str(),
        app.language_parsing.clone(),
    )
    .await
    .map(|results| Json(TokenInformationResponse { data: results }))
    .unwrap_or(Json(TokenInformationResponse { data: vec![] })))

    // Now we send over all this data to the ref/def logic
    // Ok(refdef(
    //     app.indexes.clone(),
    //     &repo_ref,
    //     &hovered_text,
    //     &range,
    //     &source_doc,
    //     &language,
    //     app.language_parsing,
    // )
    // .await
    // .map(|results| Json(TokenInformationResponse { data: results }))
    // .unwrap_or(Json(TokenInformationResponse { data: vec![] })))
}
