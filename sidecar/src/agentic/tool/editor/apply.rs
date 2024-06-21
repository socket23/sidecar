use async_trait::async_trait;

use crate::{
    agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
    chunking::text_document::Range,
};

pub struct EditorApply {
    client: reqwest::Client,
}

impl EditorApply {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EditorApplyRequest {
    fs_file_path: String,
    edited_content: String,
    selected_range: Range,
    editor_url: String,
}

impl EditorApplyRequest {
    pub fn new(
        fs_file_path: String,
        edited_content: String,
        selected_range: Range,
        editor_url: String,
    ) -> Self {
        Self {
            fs_file_path,
            edited_content,
            selected_range,
            editor_url,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EditorApplyResponse {
    fs_file_path: String,
    new_range: Range,
    success: bool,
}

impl EditorApplyResponse {
    pub fn range(&self) -> &Range {
        &self.new_range
    }
}

impl EditorApply {
    async fn apply_edits(&self, request: EditorApplyRequest) -> Result<ToolOutput, ToolError> {
        let editor_endpoint = request.editor_url.to_owned() + "/apply_edits";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&request).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let response: EditorApplyResponse = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;
        Ok(ToolOutput::EditorApplyChanges(response))
    }
}

#[async_trait]
impl Tool for EditorApply {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = input.editor_apply_changes()?;
        self.apply_edits(request).await
    }
}
