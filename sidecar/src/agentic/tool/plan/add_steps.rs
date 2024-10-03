//! We want to add steps to the plan this allows us to pick up the plan at some point
//! and add more steps if required
//!
//! Open questions: should we even show the rest of the plan, or just the prefix of the plan up until a point

use async_trait::async_trait;
use std::sync::Arc;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};

use crate::{
    agentic::{
        symbol::identifier::LLMProperties,
        tool::{
            errors::ToolError, helpers::diff_recent_changes::DiffRecentChanges, input::ToolInput,
            output::ToolOutput, r#type::Tool,
        },
    },
    user_context::types::UserContext,
};

use super::generator::StepGeneratorResponse;

#[derive(Debug, Clone)]
pub struct PlanAddRequest {
    plan_up_until_now: String,
    user_context: UserContext,
    _initial_user_query: String,
    plan_add_query: String,
    recent_edits: DiffRecentChanges,
    _editor_url: String,
    root_request_id: String,
    diagnostics: String,
    is_deep_reasoning: bool,
}

impl PlanAddRequest {
    pub fn new(
        plan_up_until_now: String,
        user_context: UserContext,
        initial_user_query: String,
        plan_add_query: String,
        recent_edits: DiffRecentChanges,
        editor_url: String,
        root_request_id: String,
        is_deep_reasoning: bool,
        diagnostics: String,
    ) -> Self {
        Self {
            plan_up_until_now,
            user_context,
            _initial_user_query: initial_user_query,
            plan_add_query,
            recent_edits,
            _editor_url: editor_url,
            root_request_id,
            is_deep_reasoning,
            diagnostics,
        }
    }
}

pub struct PlanAddStepClient {
    llm_client: Arc<LLMBroker>,
}

impl PlanAddStepClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    fn system_message(&self) -> String {
        format!(
            r#"You are an expert software engineer working alongside a developer, you take the user query and add the minimum number of steps to the plan to make sure that it satisfies the new user query.
Your job is to be precise and effective, so avoid extraneous steps even if they offer convenience. Be judicious and conservative in your planning.
Please ensure that each step includes all required fields and that the steps are logically ordered.
Since an editing system will depend your exact instructions, they must be precise. Include abridged code snippets and reasoning if it helps clarify.
- The previous part of the plan has already been executed, so we can not go back on that, we can only perform new operations.
- You are provided with the following information, use this to understand the reasoning of the changes and how to help the user.
- <plan_executed_until_now> This is the plan which we have executed until now.
- <recent_edits> These are the recent edits which we have made to the codebase already.
- <user_context> This is the context the user has provided.
- <user_current_query> This is the CURRENT USER QUERY which we want to add steps for.

You have to generate the plan in strictly the following format:
<response>
<steps>
{{There can be as many steps as you need}}
<step>
<files_to_edit>
<file>
{{File you want to edit}}
</file>
</files_to_edit>
<title>
<![CDATA[
{{The title for the change you are about to make}}
]]>
</title>
<description>
<![CDATA[
{{The description of the change you are about to make}}
]]>
</description>
</step>
</steps>
</response>
Note the use of CDATA sections within <description> and <title> to encapsulate XML-like content"#
        )
    }

    /// We want to create the update message over here and get the output in the same format
    /// For some reason this is not a core construct of ours which is weird, we should work on a structure
    /// for prompt and always parse it accordingly
    ///
    /// The format will look like this
    /// <plan_right_now>
    /// </plan_right_now>                        <- FIRST MESSAGE
    /// <recent_edits>
    /// </recent_edits>                          <- SECOND MESSAGE
    /// <user_context>
    /// </user_context>
    /// <user_current_query>
    /// </user_current_query>
    /// <reminder_about_format>
    /// </reminder_about_format>                 <- THIRD MESSAGE                 
    async fn user_message(&self, context: PlanAddRequest) -> Vec<LLMClientMessage> {
        let plan_right_now = context.plan_up_until_now;
        let user_context = context
            .user_context
            .to_xml(Default::default())
            .await
            .unwrap_or("No user context provided".to_owned());
        let diagnostics_str = context.diagnostics;
        let plan_add_query = context.plan_add_query;
        let recent_edits = context.recent_edits.to_llm_client_message();
        vec![
            LLMClientMessage::user(format!(
                r#"<plan_right_now>
{plan_right_now}
"#
            ))
            // adding cache point to the <plan_right_now>
            .cache_point(),
            LLMClientMessage::user("</plan_right_now>\n".to_owned()),
        ]
        .into_iter()
        .chain(recent_edits)
        .chain(vec![LLMClientMessage::user(format!(
            r#"<user_context>
{user_context}
</user_context>
<diagnostics>
{diagnostics_str}
</diagnostics>
<user_current_query>
{plan_add_query}
</user_current_query>
<reminder_about_format>
This is how the format should look like:
<response>
<steps>
{{There can be as many steps as you need}}
<step>
<files_to_edit>
<file>
{{File you want to edit}}
</file>
</files_to_edit>
<title>
<![CDATA[
{{The title for the change you are about to make}}
]]>
</title>
<description>
<![CDATA[
{{The description of the change you are about to make}}
]]>
</description>
</step>
</steps>
</response>
Note the use of CDATA sections within <description> and <title> to encapsulate XML-like content
</reminder_about_format>"#
        ))])
        .collect()
    }
}

#[async_trait]
impl Tool for PlanAddStepClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.is_plan_step_add()?;
        let is_deep_reasoning = context.is_deep_reasoning;
        let root_id = context.root_request_id.to_owned();
        let messages = vec![LLMClientMessage::system(self.system_message())]
            .into_iter()
            .chain(self.user_message(context).await)
            .collect::<Vec<_>>();

        let request = if is_deep_reasoning {
            LLMClientCompletionRequest::new(LLMType::O1Preview, messages, 0.2, None)
        } else {
            LLMClientCompletionRequest::new(LLMType::ClaudeSonnet, messages, 0.2, None)
        };

        let llm_properties = if is_deep_reasoning {
            LLMProperties::new(
                    LLMType::O1Preview,
                    LLMProvider::OpenAI,
                    LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("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".to_owned())),
                )
        } else {
            LLMProperties::new(
                    LLMType::ClaudeSonnet,
                    LLMProvider::Anthropic,
                    LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned())),
                )
        };

        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        let response = self
            .llm_client
            .stream_completion(
                llm_properties.api_key().clone(),
                request,
                llm_properties.provider().clone(),
                vec![
                    ("root_id".to_owned(), root_id),
                    ("event_type".to_owned(), "plan_add_step_client".to_owned()),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await?;

        let response = StepGeneratorResponse::parse_response(&response)?;

        Ok(ToolOutput::plan_add_step(response))
    }
}
