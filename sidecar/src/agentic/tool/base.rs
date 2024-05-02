//! Contains the basic tool and how to extract data from it

use std::collections::HashMap;

use axum::async_trait;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ToolType {
    AskDocumentation,
    AskUser,
    CodeEditing,
    Search,
    GoToDefinitions,
    GoToReferences,
    FileSystem,
    FolderOutline,
    Terminal,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolInput {
    tool_type: ToolType,
    metadata: HashMap<i64, String>,
}

pub enum ToolOutput {}

pub struct ToolContext {}

#[async_trait]
pub trait Tool {
    async fn invoke(input: ToolInput, context: ToolContext) -> ToolOutput;
}
