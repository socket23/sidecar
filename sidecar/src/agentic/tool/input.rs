use super::{base::ToolType, errors::ToolError};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolInput {
    tool_type: ToolType,
    metadata: String,
}

impl ToolInput {
    pub fn tool_type(&self) -> &ToolType {
        &self.tool_type
    }

    pub fn is_code_edit(&self) -> bool {
        matches!(self.tool_type, ToolType::CodeEditing)
    }

    pub fn grab_context<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<T, ToolError> {
        serde_json::from_str(&self.metadata).map_err(move |_e| ToolError::UnableToGrabContext)
    }
}
