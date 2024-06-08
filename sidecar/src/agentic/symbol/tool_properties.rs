//! This contains the configuration for the tools which can be used by the agent

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolProperties {
    swe_bench_test_endpoint: Option<String>,
}

impl ToolProperties {
    pub fn new() -> Self {
        Self {
            swe_bench_test_endpoint: None,
        }
    }

    pub fn set_swe_bench_endpoint(mut self, swe_bench_test_endpoint: Option<String>) -> Self {
        self.swe_bench_test_endpoint = swe_bench_test_endpoint;
        self
    }

    pub fn get_swe_bench_test_endpoint(&self) -> Option<String> {
        self.swe_bench_test_endpoint.clone()
    }
}
