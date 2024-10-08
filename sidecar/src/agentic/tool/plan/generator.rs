use async_trait::async_trait;
use quick_xml::de::from_str;
use serde::Deserialize;
use std::{sync::Arc, time::Instant};
use uuid::Uuid;

use llm_client::{
    broker::LLMBroker,
    clients::{
        ollama::OllamaClient,
        types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys, OllamaProvider, OpenAIProvider},
};

use crate::{
    agentic::{
        symbol::identifier::LLMProperties,
        tool::{
            errors::ToolError, input::ToolInput, lsp::file_diagnostics::DiagnosticMap,
            output::ToolOutput, r#type::Tool,
        },
    },
    user_context::types::UserContext,
};

use super::plan_step::PlanStep;

// consider possibility of constraining number of steps
#[derive(Debug, Clone)]
pub struct StepGeneratorRequest {
    user_query: String,
    user_context: Option<UserContext>,
    is_deep_reasoning: bool,
    root_request_id: String,
    editor_url: String,
    diagnostics: Option<DiagnosticMap>,
}

impl StepGeneratorRequest {
    pub fn new(
        user_query: String,
        is_deep_reasoning: bool,
        root_request_id: String,
        editor_url: String,
    ) -> Self {
        Self {
            user_query,
            root_request_id,
            editor_url,
            is_deep_reasoning,
            user_context: None,
            diagnostics: None,
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

    pub fn diagnostics(&self) -> Option<&DiagnosticMap> {
        self.diagnostics.as_ref()
    }

    pub fn with_user_context(mut self, user_context: &UserContext) -> Self {
        self.user_context = Some(user_context.to_owned());
        self
    }

    pub fn with_diagnostics(mut self, diagnostics: DiagnosticMap) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    pub fn user_context(&self) -> Option<&UserContext> {
        self.user_context.as_ref()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename = "steps")]
#[serde(rename_all = "lowercase")]
pub struct StepGeneratorResponse {
    pub step: Vec<Step>,
}

impl StepGeneratorResponse {
    pub fn into_steps(self) -> Vec<Step> {
        self.step
    }

    pub fn into_plan_steps(self) -> Vec<PlanStep> {
        let plan_steps = self
            .step
            .into_iter()
            .map(|step| step.into_plan_step())
            .collect::<Vec<_>>();

        plan_steps
    }
}

impl StepGeneratorResponse {
    pub fn parse_response(response: &str) -> Result<Self, ToolError> {
        let response = response
            .lines()
            .into_iter()
            .skip_while(|line| !line.contains("<response>"))
            .skip(1)
            .take_while(|line| !line.contains("</response>"))
            .collect::<Vec<&str>>()
            .join("\n");

        from_str::<Self>(&response).map_err(|e| {
            println!("{:?}", e);
            ToolError::SerdeConversionFailed
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Step {
    pub files_to_edit: FilesToEdit,
    pub title: String,
    pub description: String,
}

impl Step {
    pub fn into_plan_step(self) -> PlanStep {
        PlanStep::new(
            Uuid::new_v4().to_string(),
            self.files_to_edit.file,
            self.title,
            self.description,
            UserContext::new(vec![], vec![], None, vec![]),
        )
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct FilesToEdit {
    pub file: Vec<String>,
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
<file>
/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/lib.rs
</file>
<file>
/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/main.rs
</file>
</files_to_edit>
<title>
<![CDATA[
Represent Execution State if Necessary
]]>
</title>
<description>
<![CDATA[
If you need to track whether a step is paused, pending, or completed, you can introduce an ExecutionState enum:

```rs
pub struct PlanStep {{
    // ... existing fields ...
    execution_state: ExecutionState,
}}
```
Reasons for this approach:

State Management: Clearly represents the current state of the step's execution.
Extensibility: Allows for additional states in the future if needed (e.g., Failed, Skipped).
Separation of Concerns: Keeps execution state separate from other data, making the code cleaner and more maintainable.
]]>
</description>
</step>
</steps>
</response>"#
        )
    }

    pub fn system_message() -> String {
        format!(
            r#"You are a senior software engineer, expert planner and system architect.
- Given a request and context, you will generate a step by step plan to accomplish it. Use prior art seen in context where applicable.
- Your job is to be precise and effective, so avoid extraneous steps even if they offer convenience.
- Do not talk about testing out the changes unless you are instructed to do so.
- Please ensure that each step includes all required fields and that the steps are logically ordered.
- Each step you suggest must only change a single file and must be a logical unit of work, logic units of work are defined as code changes where the change is complete and encapsulates a logical step forward.
For example, if you have to import a helper function and use it in the code, it should be combined to a single step instead of it being 2 steps, one which imports the helper function and another which makes the changes.
- Do not leave placeholder code when its the critical section of the code which you know needs to change
- Since an editing system will depend your exact instructions, they must be precise. Include abridged code snippets and reasoning if it helps clarify but make sure the changes are complete and never leave core part of the logic or `// .. rest of the code` in the output
- DO NOT suggest any changes for the files which you can not see in your context.
- Your response must strictly follow the following schema:
<response>
<steps>
{{There can be as many steps as you need}}
<step>
<files_to_edit>
<file>
{{File you want to edit or CREATE a new file if required}}
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

Below we show you an example of how the output will look like:
{}

Note the use of CDATA sections within <description> and <title> to encapsulate XML-like content
"#,
            Self::plan_schema()
        )
    }

    pub async fn user_message(user_query: &str, user_context: Option<&UserContext>) -> String {
        let context_xml = match user_context {
            Some(ctx) => match ctx.to_owned().to_xml(Default::default()).await {
                Ok(xml) => xml,
                Err(e) => {
                    eprintln!("Failed to convert context to XML: {:?}", e);
                    String::from("No context")
                }
            },
            None => String::from("No context"),
        };

        format!("Context:\n{}\n---\nRequest: {}", context_xml, user_query)
    }
}

#[async_trait]
impl Tool for StepGeneratorClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = ToolInput::step_generator(input)?;

        let _editor_url = context.editor_url.to_owned();
        let root_id = context.root_request_id.to_owned();
        let is_deep_reasoning = context.is_deep_reasoning;

        let messages = vec![
            LLMClientMessage::system(Self::system_message()),
            LLMClientMessage::user(
                Self::user_message(context.user_query(), context.user_context()).await,
            ),
        ];

        // let request = if is_deep_reasoning {
        //     LLMClientCompletionRequest::new(LLMType::O1Preview, messages, 0.2, None)
        // } else {
        //     LLMClientCompletionRequest::new(LLMType::ClaudeSonnet, messages, 0.2, None)
        // };

        let request =
            LLMClientCompletionRequest::new(LLMType::Llama3_1_8bInstruct, messages, 0.2, None);

        let api_key = llm_client::provider::LLMProviderAPIKeys::Ollama(OllamaProvider {});

        let llm_properties =
            LLMProperties::new(LLMType::Llama3_1_8bInstruct, LLMProvider::Ollama, api_key);

        // let llm_properties = if is_deep_reasoning {
        //     LLMProperties::new(
        //         LLMType::O1Preview,
        //         LLMProvider::OpenAI,
        //         LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("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".to_owned())),
        //     )
        // } else {
        //     LLMProperties::new(
        //         LLMType::ClaudeSonnet,
        //         LLMProvider::Anthropic,
        //         LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned())),
        //     )
        // };
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        let start_time = Instant::now();

        let response = self
            .llm_client
            .stream_completion(
                llm_properties.api_key().clone(),
                request,
                llm_properties.provider().clone(),
                vec![
                    ("root_id".to_owned(), root_id),
                    ("event_type".to_owned(), "generate_steps".to_owned()),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await?;

        let elapsed_time = start_time.elapsed();
        println!("LLM request took: {:?}", elapsed_time);

        let response = StepGeneratorResponse::parse_response(&response)?;

        Ok(ToolOutput::StepGenerator(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response_with_cdata() {
        let input = r#"Certainly! I'll create a stepped plan to implement a new Tool called StepGeneratorClient, similar to the ReasoningClient. Here's the plan:

<response>
<steps>
<step>
<files_to_edit>
<file>
/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/plan/generator.rs
</file>
</files_to_edit>
<title>
<![CDATA[
Create StepGeneratorClient struct and implement basic methods
]]>
</title>
<description>
<![CDATA[
Create a new file `generator.rs` in the `plan` directory. Define the `StepGeneratorClient` struct and implement basic methods:

```rust
use async_trait::async_trait;
use std::sync::Arc;
use llm_client::broker::LLMBroker;

pub struct StepGeneratorClient {
    llm_client: Arc<LLMBroker>,
}

impl StepGeneratorClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    fn user_message(&self, context: StepGeneratorRequest) -> String {
        // Implement the user message formatting logic here
        // Similar to ReasoningClient's user_message method
    }
}
```
]]>
</description>
</step>

<step>
<files_to_edit>
<file>
/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/plan/generator.rs
</file>
</files_to_edit>
<title>
<![CDATA[
Define StepGeneratorRequest and StepGeneratorResponse structs
]]>
</title>
<description>
<![CDATA[
Add the following structs to `generator.rs`:

```rust
#[derive(Debug, Clone)]
pub struct StepGeneratorResponse {
    response: String,
}

impl StepGeneratorResponse {
    pub fn response(self) -> String {
        self.response
    }
}

#[derive(Debug, Clone)]
pub struct StepGeneratorRequest {
    user_query: String,
    current_plan: String,
    context: String,
    // Add other necessary fields
}

impl StepGeneratorRequest {
    pub fn new(user_query: String, current_plan: String, context: String) -> Self {
        Self {
            user_query,
            current_plan,
            context,
        }
    }
}
```
]]>
</description>
</step>

<step>
<files_to_edit>
<file>
/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/plan/generator.rs
</file>
</files_to_edit>
<title>
<![CDATA[
Implement the Tool trait for StepGeneratorClient
]]>
</title>
<description>
<![CDATA[
Implement the `Tool` trait for `StepGeneratorClient`:

```rust
use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

#[async_trait]
impl Tool for StepGeneratorClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.step_generator()?;
        
        // Implement the logic to generate steps here
        // Use self.llm_client to make API calls similar to ReasoningClient
        
        // For now, return a placeholder response
        Ok(ToolOutput::step_generator(StepGeneratorResponse {
            response: "Placeholder step generator response".to_string(),
        }))
    }
}
```
]]>
</description>
</step>

<step>
<files_to_edit>
<file>
/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/input.rs
</file>
</files_to_edit>
<title>
<![CDATA[
Update ToolInput enum to include StepGenerator
]]>
</title>
<description>
<![CDATA[
Add a new variant to the `ToolInput` enum in `input.rs`:

```rust
pub enum ToolInput {
    // ... existing variants ...
    GenerateStep(StepGeneratorRequest),
}

impl ToolInput {
    // ... existing methods ...

    pub fn step_generator(self) -> Result<StepGeneratorRequest, ToolError> {
        if let ToolInput::GenerateStep(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::StepGenerator))
        }
    }
}
```
]]>
</description>
</step>

<step>
<files_to_edit>
<file>
/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/output.rs
</file>
</files_to_edit>
<title>
<![CDATA[
Update ToolOutput enum to include StepGenerator
]]>
</title>
<description>
<![CDATA[
Add a new variant to the `ToolOutput` enum in `output.rs`:

```rust
pub enum ToolOutput {
    // ... existing variants ...
    StepGenerator(StepGeneratorResponse),
}

impl ToolOutput {
    // ... existing methods ...

    pub fn step_generator(response: StepGeneratorResponse) -> Self {
        ToolOutput::StepGenerator(response)
    }

    pub fn get_step_generator_output(self) -> Option<StepGeneratorResponse> {
        match self {
            ToolOutput::StepGenerator(response) => Some(response),
            _ => None,
        }
    }
}
```
]]>
</description>
</step>

<step>
<files_to_edit>
<file>
/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/type.rs
</file>
</files_to_edit>
<title>
<![CDATA[
Update ToolType enum to include StepGenerator
]]>
</title>
<description>
<![CDATA[
Add a new variant to the `ToolType` enum in `type.rs`:

```rust
pub enum ToolType {
    // ... existing variants ...
    StepGenerator,
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... existing matches ...
            ToolType::StepGenerator => write!(f, "Step generator"),
        }
    }
}
```
]]>
</description>
</step>

<step>
<files_to_edit>
<file>
/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/broker.rs
</file>
</files_to_edit>
<title>
<![CDATA[
Update ToolBroker to include StepGeneratorClient
]]>
</title>
<description>
<![CDATA[
Update the `ToolBroker::new` method in `broker.rs` to include the `StepGeneratorClient`:

```rust
use super::plan::generator::StepGeneratorClient;

impl ToolBroker {
    pub fn new(
        // ... existing parameters ...
    ) -> Self {
        let mut tools: HashMap<ToolType, Box<dyn Tool + Send + Sync>> = Default::default();
        
        // ... existing tool insertions ...

        tools.insert(
            ToolType::StepGenerator,
            Box::new(StepGeneratorClient::new(llm_client.clone())),
        );

        // ... rest of the method ...
    }
}
```
]]>
</description>
</step>
</steps>
</response>

This plan outlines the steps to create a new `StepGeneratorClient` tool, similar to the `ReasoningClient`. It includes creating the necessary structs, implementing the `Tool` trait, and updating the relevant enums and broker to include the new tool. You can follow these steps to implement the `StepGeneratorClient` in your project."#;
        let result = StepGeneratorResponse::parse_response(input);

        assert!(result.is_ok());
        // let response = result.unwrap();
    }
}
