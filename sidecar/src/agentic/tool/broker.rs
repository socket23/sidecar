use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

use llm_client::broker::LLMBroker;

use super::{
    base::{Tool, ToolType},
    code_edit::types::CodeEditingTool,
    errors::ToolError,
    input::ToolInput,
    output::ToolOutput,
};

// TODO(skcd): We want to use a different serializer and deserializer for this
// since we are going to be storing an array of tools over here, we have to make
// sure that we do not store everything about the tool but a representation of it
pub struct ToolBroker {
    tools: HashMap<ToolType, Box<dyn Tool + Send + Sync>>,
}

impl ToolBroker {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        let mut tools: HashMap<ToolType, Box<dyn Tool + Send + Sync>> = Default::default();
        tools.insert(
            ToolType::CodeEditing,
            Box::new(CodeEditingTool::new(llm_client)),
        );
        Self { tools }
    }
}

#[async_trait]
impl Tool for ToolBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let tool_type = input.tool_type();
        if let Some(tool) = self.tools.get(tool_type) {
            tool.invoke(input).await
        } else {
            Err(ToolError::MissingTool)
        }
    }
}
