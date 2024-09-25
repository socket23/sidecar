use async_trait::async_trait;
use std::sync::Arc;

use llm_client::broker::LLMBroker;

use crate::{
    agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
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
            r#"<steps>
<step>
<index>
0
</index>
<files_to_edit>
<file>src/main.rs</file>
<file>src/lib.rs</file>
</files_to_edit>
<description>Update the main function to include error handling</description>
</step>
</steps>"#
        )
    }

    pub fn system_message() -> String {
        format!(
            r#"You are a senior software engineer, expert planner and system architect.

Given a request and context, you will generate a step by step plan to accomplish it:

Please ensure that each step includes all required fields and that the steps are logically ordered.

The plan must be structured as per the following schema:

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
}

#[async_trait]
impl Tool for StepGeneratorClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        todo!();
    }
}
