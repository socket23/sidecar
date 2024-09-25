use async_trait::async_trait;
use quick_xml::de::from_str;
use std::sync::Arc;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};

use crate::{
    agentic::{
        symbol::identifier::LLMProperties,
        tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
    },
    user_context::types::UserContext,
};

// consider possibility of constraining number of steps
#[derive(Debug, Clone)]
pub struct StepGeneratorRequest {
    user_query: String,
    user_context: Option<UserContext>,
    root_request_id: String,
    editor_url: String,
}

impl StepGeneratorRequest {
    pub fn new(user_query: String, root_request_id: String, editor_url: String) -> Self {
        Self {
            user_query,
            root_request_id,
            editor_url,
            user_context: None,
        }
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

    pub fn with_user_context(mut self, user_context: UserContext) -> Self {
        self.user_context = Some(user_context);
        self
    }

    pub fn user_context(&self) -> Option<&UserContext> {
        self.user_context.as_ref()
    }
}

pub struct StepGeneratorClient {
    llm_client: Arc<LLMBroker>,
}

impl StepGeneratorClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    pub fn plan_schema() -> String {
        format!(
            r#"<response>
<steps>
<step>
<files_to_edit>
<file>/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/lib.rs</file>
<file>/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/main.rs</file>
</files_to_edit>
<title>Represent Execution State if Necessary</title>
<description>If you need to track whether a step is paused, pending, or completed, you can introduce an ExecutionState enum:

```rs
pub struct PlanStep {{
    // ... existing fields ...
    execution_state: ExecutionState,
}}
```
Reasons for this approach:

State Management: Clearly represents the current state of the step's execution.
Extensibility: Allows for additional states in the future if needed (e.g., Failed, Skipped).
Separation of Concerns: Keeps execution state separate from other data, making the code cleaner and more maintainable.</description>
</step>
</steps>
</response>"#
        )
    }

    pub fn system_message() -> String {
        format!(
            r#"You are a senior software engineer, expert planner and system architect.

Given a request and context, you will generate a step by step plan to accomplish it. Use prior art seen in context where applicable.

Please ensure that each step includes all required fields and that the steps are logically ordered.

Since an editing system will depend your exact instructions, they must be precise. Include abridged code snippets and reasoning if it helps clarify.

Your response must strictly follow the following schema:
{}
"#,
            Self::plan_schema()
        )
    }

    pub async fn user_message(user_query: &str, user_context: Option<&UserContext>) -> String {
        let context_xml_res = match user_context {
            Some(ctx) => ctx.to_owned().to_xml(Default::default()).await,
            None => Ok(String::from("No context")),
        };

        let context_xml = match context_xml_res {
            Ok(xml) => xml,
            Err(e) => {
                println!("step_generator_client::user_message::err(Failed to convert context to XML: {:?})", e);
                String::from("No context")
            }
        };

        format!("Context:\n{}\n---\nRequest: {}", context_xml, user_query)
    }

    // this should be on the response struct...
    //     fn parse_response(response: &str) -> Result<Self, ToolError> {
    //         let lines = response
    //             .lines()
    //             .skip_while(|line| !line.contains("<reply>"))
    //             .skip(1)
    //             .take_while(|line| !line.contains("</reply>"))
    //             .collect::<Vec<_>>()
    //             .join("\n");
    //         let formatted_lines = format!(
    //             r#"<reply>
    // {lines}
    // </reply>"#
    //         );
    //         from_str::<Self>(&formatted_lines).map_err(|_e| ToolError::SerdeConversionFailed)
    //     }
}

#[async_trait]
impl Tool for StepGeneratorClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = ToolInput::step_generator(input)?;

        let _editor_url = context.editor_url.to_owned();
        let root_id = context.root_request_id.to_owned();

        let messages = vec![
            LLMClientMessage::system(Self::system_message()),
            LLMClientMessage::user(
                Self::user_message(context.user_query(), context.user_context()).await,
            ),
        ];

        let request = LLMClientCompletionRequest::new(LLMType::ClaudeSonnet, messages, 0.2, None);

        // todo(zi): this could be o1
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
                    (
                        "event_type".to_owned(),
                        format!("generate_steps").to_owned(),
                    ),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await?;

        dbg!(response);

        todo!();
    }
}
