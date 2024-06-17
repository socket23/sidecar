//! We might have multiple questions which we want to ask the code symbol
//! from a previous symbol (if they all lead to the same symbol)
//! The best way to handle this is to let a LLM figure out what is the best
//! question we can ask the symbol

use async_trait::async_trait;
use std::sync::Arc;

use llm_client::broker::LLMBroker;

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput},
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProbeQuestionForSymbolRequest {
    previous_symbol_name: String,
    current_symbol_name: String,
    current_symbol_file_path: String,
    // all the places we are linked with the current symbol
    hyperlinks: Vec<String>,
}

pub struct ProbeQuestionForSymbol {
    llm_client: Arc<LLMBroker>,
    fallback_llm: LLMProperties,
}

impl ProbeQuestionForSymbol {
    pub fn new(llm_client: Arc<LLMBroker>, fallback_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            fallback_llm,
        }
    }

    fn system_message(&self) -> String {
        format!(r#""#)
    }

    fn user_message(&self, request: ProbeQuestionForSymbolRequest) -> String {
        format!(r#""#)
    }
}

#[async_trait]
impl Tool for ProbeQuestionForSymbol {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.get_probe_create_question_for_symbol()?;
        todo!("figure out how to implement this")
    }
}
