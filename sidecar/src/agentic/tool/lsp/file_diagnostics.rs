use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::diagnostics::{Diagnostic, LSPDiagnosticsOutput};
use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

pub struct FileDiagnostics {
    client: Client,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileDiagnosticsInput {
    fs_file_path: String,
    editor_url: String,
}

impl FileDiagnosticsInput {
    pub fn new(fs_file_path: String, editor_url: String) -> Self {
        Self {
            fs_file_path,
            editor_url,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileDiagnosticsOutput {
    diagnostics: Vec<Diagnostic>,
}

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
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let diagnostics_response: FileDiagnosticsOutput = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;

        Ok(ToolOutput::file_diagnostics(diagnostics_response))
    }
}
