//! We want to get the diagnostics which might be present on a file after making
//! the edit, this is extremely useful to verify if the code written has produced
//! any errors. We have to time when the LSP is ready for providing the diagnostics
//! cause there is no clear way to do that in VScode, as its all async right now
//!
//! Note: we do not store the editor url here since we could have reloaded the editor
//! and the url changes because of that

use async_trait::async_trait;

use crate::{
    agentic::tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput},
    chunking::text_document::Range,
};

pub struct LSPDiagnostics {
    client: reqwest::Client,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct LSPDiagnosticsInput {
    fs_file_path: String,
    range: Range,
    editor_url: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Diagnostic {
    diagnostic: String,
    range: Range,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct LSPDiagnosticsOutput {
    diagnostics: Vec<Diagnostic>,
}

impl LSPDiagnostics {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for LSPDiagnostics {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_lsp_diagnostics()?;
        let editor_endpoint = context.editor_url.to_owned() + "/diagnostics";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;

        let diagnostics_response: LSPDiagnosticsOutput = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;

        Ok(ToolOutput::lsp_diagnostics(diagnostics_response))
    }
}
