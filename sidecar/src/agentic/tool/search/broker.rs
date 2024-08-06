use std::{collections::HashMap, sync::Arc};

use llm_client::{broker::LLMBroker, clients::types::LLMType};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

use async_trait::async_trait;

use super::{agentic::GenerateSearchPlan, google_studio::GoogleStudioPlanGenerator};

pub struct SearchPlanBroker {
    llms: HashMap<LLMType, Box<dyn GenerateSearchPlan + Send + Sync>>,
}

impl SearchPlanBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        let mut llms: HashMap<LLMType, Box<dyn GenerateSearchPlan + Send + Sync>> =
            Default::default();
        llms.insert(
            LLMType::GeminiProFlash,
            Box::new(GoogleStudioPlanGenerator::new(
                llm_client.clone(),
                fail_over_llm.clone(),
            )),
        );

        Self { llms }
    }
}

#[async_trait]
impl Tool for SearchPlanBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = input.search_plan_query()?;

        if let Some(implementation) = self.llms.get(request.llm()) {
            let response = implementation
                .generate_search_plan(&request)
                .await
                .map_err(|e| ToolError::SearchPlanError(e))?;

            Ok(ToolOutput::SearchPlan(response))
        } else {
            Err(ToolError::LLMNotSupported)
        }
    }
}
