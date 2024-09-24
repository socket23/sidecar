use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};
use std::sync::Arc;

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

use super::plan::Plan;

#[derive(Debug)]
pub struct PlanUpdateRequest {
    plan: Plan,
    new_context: String,
    checkpoint_index: usize,
    user_query: String,
    root_request_id: String,
    editor_url: String,
}

impl PlanUpdateRequest {
    pub fn new(
        plan: Plan,
        new_context: String,
        checkpoint_index: usize,
        user_query: String,
        root_request_id: String,
        editor_url: String,
    ) -> Self {
        Self {
            plan,
            new_context,
            checkpoint_index,
            user_query,
            root_request_id,
            editor_url,
        }
    }

    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    pub fn new_context(&self) -> &str {
        &self.new_context
    }

    pub fn checkpoint_index(&self) -> usize {
        self.checkpoint_index
    }

    pub fn user_query(&self) -> &str {
        &self.user_query
    }

    pub fn root_request_id(&self) -> &str {
        &self.root_request_id
    }

    pub fn editor_url(&self) -> &str {
        &self.editor_url
    }
}

pub struct PlanUpdaterClient {
    llm_client: Arc<LLMBroker>,
}

impl PlanUpdaterClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }
}

#[async_trait]
impl Tool for PlanUpdaterClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        // check whether tool_input is for plan updater
        let context = input.plan_updater()?;

        let editor_url = context.editor_url.to_owned();
        let root_id = context.root_request_id.to_owned();

        // construct messages

        let request = LLMClientCompletionRequest::new(
            LLMType::ClaudeSonnet,
            vec![LLMClientMessage::user(self.user_message(context))],
            0.2,
            None,
        );

        let llm_properties = LLMProperties::new(
            LLMType::ClaudeSonnet,
            LLMProvider::Anthropic,
            LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned())),
        );
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        let response = self
            .llm_client
            .stream_completion(
                llm_properties.api_key().clone(),
                request,
                llm_properties.provider().clone(),
                vec![
                    ("root_id".to_owned(), root_id),
                    ("event_type".to_owned(), format!("update_plan").to_owned()),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await;

        // LLM call

        // parse
        todo!()
    }
}
