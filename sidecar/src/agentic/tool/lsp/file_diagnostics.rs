use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::diagnostics::Diagnostic;
use crate::agentic::{
    symbol::events::lsp::LSPDiagnosticError,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

pub struct FileDiagnostics {
    client: Client,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileDiagnosticsInput {
    fs_file_path: String,
    editor_url: String,
    with_enrichment: bool,
}

impl FileDiagnosticsInput {
    pub fn new(fs_file_path: String, editor_url: String, with_enrichment: bool) -> Self {
        Self {
            fs_file_path,
            editor_url,
            with_enrichment,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileDiagnosticsOutput {
    diagnostics: Vec<Diagnostic>,
}

/// Diagnostics grouped by fs_file_path
pub type DiagnosticMap = HashMap<String, Vec<LSPDiagnosticError>>;

impl FileDiagnosticsOutput {
    pub fn get_diagnostics(&self) -> &[Diagnostic] {
        self.diagnostics.as_slice()
    }

    pub fn remove_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

impl FileDiagnostics {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl Tool for FileDiagnostics {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_file_diagnostics()?;
        let editor_endpoint = context.editor_url.to_owned() + "/file_diagnostics";
        let response = self
            .client
            .post(editor_endpoint)
            .json(&context)
            .send()
            .await
            .map_err(|e| {
                eprintln!("{:?}", e);
                ToolError::ErrorCommunicatingWithEditor
            })?;

        let diagnostics_response: FileDiagnosticsOutput = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;

        Ok(ToolOutput::file_diagnostics(diagnostics_response))
    }
}
