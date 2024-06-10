//! Contains the required logic for supporting test correction logic through
//! code editing

use async_trait::async_trait;
use std::sync::Arc;

use llm_client::broker::LLMBroker;

use crate::{
    agentic::tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput},
    chunking::text_document::Range,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct TestOutputCorrectionRequest {
    fs_file_path: String,
    file_contents: String,
    edited_range: Range,
    original_code: String,
    language: String,
    test_output_logs: String,
}

impl TestOutputCorrectionRequest {
    pub fn new(
        fs_file_path: String,
        file_contents: String,
        edited_range: Range,
        original_code: String,
        language: String,
        test_output_logs: String,
    ) -> Self {
        Self {
            fs_file_path,
            file_contents,
            edited_range,
            original_code,
            language,
            test_output_logs,
        }
    }
}

pub struct TestCorrection {
    llm_client: Arc<LLMBroker>,
}

impl TestCorrection {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }
}

#[async_trait]
impl Tool for TestCorrection {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_test_output()?;
        todo!()
    }
}
