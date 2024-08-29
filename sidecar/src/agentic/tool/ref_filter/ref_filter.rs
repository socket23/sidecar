use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};
use std::sync::Arc;

use crate::agentic::{
    symbol::identifier::{LLMProperties, SymbolIdentifier},
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReferenceFilterRequest {
    user_instruction: String,
    references: Vec<SymbolIdentifier>, // todo(zi) this needs to be considered.
    llm_properties: LLMProperties,
    root_id: String,
}

impl ReferenceFilterRequest {
    pub fn new(
        user_instruction: String,
        references: Vec<SymbolIdentifier>,
        llm_properties: LLMProperties,
        root_id: String,
    ) -> Self {
        Self {
            user_instruction,
            references,
            llm_properties,
            root_id,
        }
    }

    pub fn references(&self) -> &[SymbolIdentifier] {
        &self.references
    }

    pub fn user_instruction(&self) -> &str {
        &self.user_instruction
    }

    pub fn llm_properties(&self) -> &LLMProperties {
        &self.llm_properties
    }

    pub fn root_id(&self) -> &str {
        &self.root_id
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReferenceFilterResponse {
    references: Vec<SymbolIdentifier>,
}

impl ReferenceFilterResponse {
    pub fn new(references: Vec<SymbolIdentifier>) -> Self {
        Self { references }
    }

    pub fn references(&self) -> &[SymbolIdentifier] {
        &self.references
    }
}

pub struct ReferenceFilterBroker {
    llm_client: Arc<LLMBroker>,
    fail_over_llm: LLMProperties,
}

impl ReferenceFilterBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            fail_over_llm,
        }
    }

    pub fn system_message(&self) -> String {
        format!(r#"test"#)
    }

    pub fn user_message(&self, request: &ReferenceFilterRequest) -> String {
        let references = request.references();
        let user_query = request.user_instruction();
        format!(r#"test"#)
    }
}

#[async_trait]
impl Tool for ReferenceFilterBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.filter_references_request()?;
        let llm_properties = context.llm_properties.clone();
        let root_request_id = context.root_id.to_owned();

        let system_message = LLMClientMessage::system(self.system_message());
        let user_message = LLMClientMessage::user(self.user_message(&context));

        let request = LLMClientCompletionRequest::new(
            llm_properties.llm().clone(),
            vec![system_message, user_message],
            0.2,
            None,
        );

        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        let response = self
            .llm_client
            .stream_completion(
                llm_properties.api_key().clone(),
                request,
                llm_properties.provider().clone(),
                vec![
                    ("event_type".to_owned(), "filter_references".to_owned()),
                    ("root_id".to_owned(), root_request_id.to_owned()),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await
            .map_err(|e| ToolError::LLMClientError(e))?;

        dbg!(&response);

        todo!();
    }
}
