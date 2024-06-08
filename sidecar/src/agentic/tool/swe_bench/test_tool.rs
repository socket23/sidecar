//! Contains the test tool

use crate::agentic::tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput};
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SWEBenchTestRequest {
    swe_bench_test_endpoint: String,
}

impl SWEBenchTestRequest {
    pub fn new(swe_bench_test_endpoint: String) -> Self {
        Self {
            swe_bench_test_endpoint,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SWEBenchTestRepsonse {
    test_output: String,
}

pub struct SWEBenchTestTool {
    client: reqwest::Client,
}

impl SWEBenchTestTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for SWEBenchTestTool {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.swe_bench_test()?;
        let response = self
            .client
            .post(context.swe_bench_test_endpoint.to_owned())
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::SWEBenchTestEndpointError)?;
        let response: SWEBenchTestRepsonse = response
            .json()
            .await
            .map_err(|_e| ToolError::SerdeConversionFailed)?;
        Ok(ToolOutput::swe_bench_test_output(response.test_output))
    }
}
