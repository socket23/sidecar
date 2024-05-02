use super::base::{ToolInput, ToolOutput};

// TODO(skcd): We want to use a different serializer and deserializer for this
// since we are going to be storing an array of tools over here, we have to make
// sure that we do not store everything about the tool but a representation of it
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolBroker {}

impl ToolBroker {
    pub fn execute_tool(&self, tool_input: ToolInput) -> ToolOutput {
        todo!("we are going to finish this up")
    }
}
