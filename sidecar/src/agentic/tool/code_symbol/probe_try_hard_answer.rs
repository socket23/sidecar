use async_trait::async_trait;
use std::sync::Arc;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeTryHardAnswerSymbolRequest {
    original_request: String,
    probe_request: String,
    symbol_content: String,
    llm_properties: LLMProperties,
}

impl ProbeTryHardAnswerSymbolRequest {
    pub fn new(
        original_request: String,
        probe_request: String,
        symbol_content: String,
        llm_properties: LLMProperties,
    ) -> Self {
        Self {
            original_request,
            probe_request,
            symbol_content,
            llm_properties,
        }
    }
}

pub struct ProbeTryHardAnswer {
    llm_client: Arc<LLMBroker>,
}

impl ProbeTryHardAnswer {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    fn system_message(&self) -> String {
        r#"You are an expert software engineer who is helping a user explore the codebase. During the exploration we have reached a point where there are no further code symbols to follow and we have to reply to the user.
- The original question which the user had asked when we started exploring the codebase is given in <user_query>.
- The question which we are asking at this point in the codebase to the current code in selection is given in <current_question>.
- You have to look at the code provided to you in <code> section and create a reply for the user. The reply should be at the most 100 words and be concise."#.to_owned()
    }

    fn user_message(&self, request: ProbeTryHardAnswerSymbolRequest) -> String {
        let user_query = request.original_request;
        let current_question = request.probe_request;
        let symbol_content = request.symbol_content;
        format!(
            r#"<user_query>
{user_query}
</user_query>

<current_question>
{current_question}
</current_question>

<code>
{symbol_content}
</code>"#
        )
    }
}

#[async_trait]
impl Tool for ProbeTryHardAnswer {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.get_probe_try_hard_answer_request()?;
        let llm_properties = context.llm_properties.clone();
        let system_message = LLMClientMessage::system(self.system_message());
        let user_message = LLMClientMessage::user(self.user_message(context));
        let llm_request = LLMClientCompletionRequest::new(
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
                llm_request,
                llm_properties.provider().clone(),
                vec![(
                    "event_type".to_owned(),
                    "probe_try_hard_to_answer".to_owned(),
                )]
                .into_iter()
                .collect(),
                sender,
            )
            .await
            .map_err(|e| ToolError::LLMClientError(e))?;
        Ok(ToolOutput::ProbeTryHardAnswer(response))
    }
}
