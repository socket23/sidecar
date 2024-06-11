//! This contains the configuration for the tools which can be used by the agent

use super::identifier::LLMProperties;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolProperties {
    swe_bench_test_endpoint: Option<String>,
    swe_bench_code_editing_llm: Option<LLMProperties>,
}

impl ToolProperties {
    pub fn new() -> Self {
        Self {
            swe_bench_test_endpoint: None,
            swe_bench_code_editing_llm: None,
        }
    }

    pub fn set_swe_bench_code_editing_llm(
        mut self,
        swe_bench_code_editing_llm: Option<LLMProperties>,
    ) -> Self {
        self.swe_bench_code_editing_llm = swe_bench_code_editing_llm;
        self
    }

    pub fn set_swe_bench_endpoint(mut self, swe_bench_test_endpoint: Option<String>) -> Self {
        self.swe_bench_test_endpoint = swe_bench_test_endpoint;
        self
    }

    pub fn get_swe_bench_test_endpoint(&self) -> Option<String> {
        self.swe_bench_test_endpoint.clone()
    }

    pub fn get_swe_bench_code_editing_llm(&self) -> Option<LLMProperties> {
        self.swe_bench_code_editing_llm.clone()
    }
}
