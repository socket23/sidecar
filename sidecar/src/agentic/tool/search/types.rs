use async_trait::async_trait;

use crate::agentic::tool::{
    code_symbol::{important::CodeSymbolImportantResponse, types::CodeSymbolError},
    errors::ToolError,
    input::ToolInput,
    output::ToolOutput,
    r#type::Tool,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchType {
    Tree(String),
    Repomap(String),
    Both(String, String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BigSearchRequest {}

impl BigSearchRequest {}

#[async_trait]
pub trait BigSearch {
    async fn search(
        &self,
        input: BigSearchRequest,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError>;
}

pub struct BigSearchBroker {
    strategies: Vec<Box<dyn BigSearch + Send + Sync>>,
}

impl BigSearchBroker {
    pub fn new() -> Self {
        Self { strategies: vec![] }
    }

    pub fn with_strategy(self, strategy: impl BigSearch + Send + Sync + 'static) -> Self {
        Self {
            strategies: {
                let mut strategies = self.strategies;
                strategies.push(Box::new(strategy));
                strategies
            },
        }
    }

    pub async fn search(
        &self,
        input: BigSearchRequest,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError> {
        let mut output: Vec<CodeSymbolImportantResponse> = vec![];
        for strategy in &self.strategies {
            let result = strategy.search(input.clone()).await;
            if let Ok(result) = result {
                println!("BigSearchBroker::search::strategy::search: {:?}", result);
                output.push(result);
            }
        }

        let output = CodeSymbolImportantResponse::merge(output);

        println!("BigSearchBroker::search::output: {:?}", output);

        Err(CodeSymbolError::NoStrategyFound)
    }
}

#[async_trait]
impl Tool for BigSearchBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = match input {
            ToolInput::BigSearch(req) => req,
            _ => {
                return Err(ToolError::BigSearchError(
                    "Expected BigSearch input".to_string(),
                ))
            }
        };

        let result = self
            .search(request)
            .await
            .map_err(|e| ToolError::CodeSymbolError(e))?;

        Ok(ToolOutput::BigSearch(result))
    }
}
