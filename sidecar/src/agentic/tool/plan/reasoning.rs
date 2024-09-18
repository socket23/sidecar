//! Reasoning tool, we just show it all the information we can and ask it for a query
//! to come up with a plan and thats it

use async_trait::async_trait;
use std::sync::Arc;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

#[derive(Debug, Clone)]
pub struct ReasoningResponse {
    response: String,
}

impl ReasoningResponse {
    pub fn response(self) -> String {
        self.response
    }
}

#[derive(Debug, Clone)]
pub struct ReasoningRequest {
    user_query: String,
    files_in_selection: String,
    code_in_selection: String,
    lsp_diagnostics: String,
    diff_recent_edits: String,
    root_request_id: String,
}

impl ReasoningRequest {
    pub fn new(
        user_query: String,
        files_in_selection: String,
        code_in_selection: String,
        lsp_diagnostics: String,
        diff_recent_edits: String,
        root_request_id: String,
    ) -> Self {
        Self {
            user_query,
            files_in_selection,
            code_in_selection,
            lsp_diagnostics,
            diff_recent_edits,
            root_request_id,
        }
    }
}

pub struct ReasoningClient {
    llm_client: Arc<LLMBroker>,
}

impl ReasoningClient {
    fn user_message(&self, context: ReasoningRequest) -> String {
        let user_query = context.user_query;
        let files_in_selection = context.files_in_selection;
        let code_in_selection = context.code_in_selection;
        let lsp_diagnostics = context.lsp_diagnostics;
        let diff_recent_edits = context.diff_recent_edits;
        format!(
            r#"<files_in_selection>
{files_in_selection}
</files_in_selection>
<recent_diff_edits>
{diff_recent_edits}
</recent_diff_edits>
<lsp_diagnostics>
{lsp_diagnostics}
</lsp_diagnostics>
<code_in_selection>
{code_in_selection}
</code_in_selection>

I have provided you with the following context:
- <files_in_selection>
These are the files which are present in context that is useful
- <recent_diff_edits>
The recent edits which have been made to the files
- <lsp_diagnostics>
The diagnostic errors which are generated from the Language Server running inside the editor
- <code_in_selection>
These are the code sections which are in our selection

The query I want help with:
{user_query}"#
        )
    }
}

#[async_trait]
impl Tool for ReasoningClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.should_reasoning()?;
        let root_id = context.root_request_id.to_owned();
        let request = LLMClientCompletionRequest::new(
            LLMType::O1Preview,
            vec![LLMClientMessage::user(self.user_message(context))],
            1.0,
            None,
        );
        let llm_properties = LLMProperties::new(
            LLMType::O1Preview,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("sk-GF8nCfhNTszdK_rr96cxH2vNEQw6aLa4V5FhTka80aT3BlbkFJWS6GYYDuNGSDwqjEuZTSDG2R2EYcHPp14mx8DL6HIA".to_owned())),
        );
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let model_str = llm_properties.llm().to_string();
        let response = self
            .llm_client
            .stream_completion(
                llm_properties.api_key().clone(),
                request,
                llm_properties.provider().clone(),
                vec![
                    ("root_id".to_owned(), root_id),
                    (
                        "event_type".to_owned(),
                        format!("reasoning_{}", model_str).to_owned(),
                    ),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await;
        response
            .map(|response| ToolOutput::reasoning(ReasoningResponse { response }))
            .map_err(|e| ToolError::LLMClientError(e))
    }
}
