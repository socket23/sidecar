//! Contains the output of a tool which can be used by any of the callers
pub enum ToolOutput {
    CodeEditTool(String),
}

impl ToolOutput {
    pub fn code_edit_output(output: String) -> Self {
        ToolOutput::CodeEditTool(output)
    }
}
