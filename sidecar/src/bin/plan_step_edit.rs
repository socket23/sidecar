use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{
        AnthropicAPIKey, FireworksAPIKey, GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys,
        OpenAIProvider,
    },
};
use sidecar::{
    agentic::{
        symbol::{
            events::{
                input::{SymbolEventRequestId, SymbolInputEvent},
                message_event::SymbolEventMessageProperties,
            },
            identifier::LLMProperties,
            manager::SymbolManager,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
            plan::{plan::Plan, plan_step::PlanStep},
        },
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    user_context::types::UserContext,
};

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    let request_id = uuid::Uuid::new_v4();
    let request_id_str = request_id.to_string();
    let parea_url = format!(
        r#"https://app.parea.ai/logs?colViz=%7B%220%22%3Afalse%2C%221%22%3Afalse%2C%222%22%3Afalse%2C%223%22%3Afalse%2C%22error%22%3Afalse%2C%22deployment_id%22%3Afalse%2C%22feedback_score%22%3Afalse%2C%22time_to_first_token%22%3Afalse%2C%22scores%22%3Afalse%2C%22start_timestamp%22%3Afalse%2C%22user%22%3Afalse%2C%22session_id%22%3Afalse%2C%22target%22%3Afalse%2C%22experiment_uuid%22%3Afalse%2C%22dataset_references%22%3Afalse%2C%22in_dataset%22%3Afalse%2C%22event_type%22%3Afalse%2C%22request_type%22%3Afalse%2C%22evaluation_metric_names%22%3Afalse%2C%22request%22%3Afalse%2C%22calling_node%22%3Afalse%2C%22edges%22%3Afalse%2C%22metadata_evaluation_metric_names%22%3Afalse%2C%22metadata_event_type%22%3Afalse%2C%22metadata_0%22%3Afalse%2C%22metadata_calling_node%22%3Afalse%2C%22metadata_edges%22%3Afalse%2C%22metadata_root_id%22%3Afalse%7D&filter=%7B%22filter_field%22%3A%22meta_data%22%2C%22filter_operator%22%3A%22equals%22%2C%22filter_key%22%3A%22root_id%22%2C%22filter_value%22%3A%22{request_id_str}%22%7D&page=1&page_size=50&time_filter=1m"#
    );
    println!("===========================================\nRequest ID: {}\nParea AI: {}\n===========================================", request_id.to_string(), parea_url);
    let editor_url = "http://localhost:42425".to_owned();
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
    let anthropic_llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys.clone(),
    );
    let editor_parsing = Arc::new(EditorParsing::default());
    let symbol_broker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));
    let tool_broker = Arc::new(ToolBroker::new(
        Arc::new(
            LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
                .await
                .expect("to initialize properly"),
        ),
        Arc::new(CodeEditBroker::new()),
        symbol_broker.clone(),
        Arc::new(TSLanguageParsing::init()),
        // for our testing workflow we want to apply the edits directly
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::Gpt4O,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
                "sk-proj-BLaSMsWvoO6FyNwo9syqT3BlbkFJo3yqCyKAxWXLm4AvePtt".to_owned(),
            )),
        ),
    ));

    let user_context = UserContext::new(vec![], vec![], None, vec![]);

    let (sender, mut _receiver) = tokio::sync::mpsc::unbounded_channel();

    let _event_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new("".to_owned(), "".to_owned()),
        sender.clone(),
        editor_url.to_owned(),
        tokio_util::sync::CancellationToken::new(),
    );

    let _symbol_manager = SymbolManager::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing,
        anthropic_llm_properties.clone(),
    );

    let problem_statement = "add a new field user_id to the Tag struct".to_owned();

    let root_dir = "/Users/zi/codestory/sidecar/sidecar/src";

    let _initial_request = SymbolInputEvent::new(
        user_context,
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys,
        problem_statement,
        request_id.to_string(),
        request_id.to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        true, // full code editing
        Some(root_dir.to_string()),
        None,
        true, // big_search
        sender,
    );

    let initial_context = "Create a CLI tool in Rust".to_string();
    let user_query = "I want to build a todo list application".to_string();

    let steps = vec![
        r#"Step 1: Define a New ToolType Variant
File: tool/type.rs

Add a new variant to the ToolType enum:

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ToolType {
    // ... existing variants ...
    CodeSummarization,
}"#
        .to_string(),
        r#"Step 2: Define a New ToolInput Variant
File: tool/input.rs

Add a new variant to the ToolInput enum:

#[derive(Debug, Clone)]
pub enum ToolInput {
    // ... existing variants ...
    CodeSummarization(CodeSummarizationRequest),
}"#
        .to_string(),
        r#"Step 3: Create CodeSummarizationRequest Struct
File: tool/code_summarization.rs (new file)

Define the input structure for the summarization tool:

#[derive(Debug, Clone)]
pub struct CodeSummarizationRequest {
    pub code: String,
    pub root_request_id: String,
}"#
        .to_string(),
        r#"Step 4: Create CodeSummarizationResponse Struct
File: tool/code_summarization.rs (same file as above)

Define the output structure for the summarization tool:

#[derive(Debug, Clone)]
pub struct CodeSummarizationResponse {
    pub summary: String,
}"#
        .to_string(),
        r#"Step 5: Implement Methods in ToolInput for the New Variant
File: tool/input.rs

Add a method to extract CodeSummarizationRequest:

impl ToolInput {
    // ... existing methods ...

    pub fn should_code_summarization(self) -> Result<CodeSummarizationRequest, ToolError> {
        if let ToolInput::CodeSummarization(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::CodeSummarization))
        }
    }
}"#
        .to_string(),
        r#"Step 6: Implement Methods in ToolOutput for the New Variant
File: tool/output.rs

Add a new variant to the ToolOutput enum:

#[derive(Debug)]
pub enum ToolOutput {
    // ... existing variants ...
    CodeSummarization(CodeSummarizationResponse),
}

Add methods to handle the new variant:

impl ToolOutput {
    // ... existing methods ...

    pub fn code_summarization(response: CodeSummarizationResponse) -> Self {
        ToolOutput::CodeSummarization(response)
    }

    pub fn get_code_summarization(self) -> Option<CodeSummarizationResponse> {
        match self {
            ToolOutput::CodeSummarization(response) => Some(response),
            _ => None,
        }
    }
}"#
        .to_string(),
        r#"Step 7: Implement the Tool Trait for SummaryClient
File: tool/code_summarization.rs

Implement the Tool trait for SummaryClient:

use async_trait::async_trait;
use std::sync::Arc;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};
use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

pub struct SummaryClient {
    llm_client: Arc<LLMBroker>,
}

impl SummaryClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    fn user_message(&self, request: &CodeSummarizationRequest) -> String {
        format!(
            "Please provide a concise summary of the following code:\n\n{}",
            request.code
        )
    }
}

#[async_trait]
impl Tool for SummaryClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = input.should_code_summarization()?;

        let llm_request = LLMClientCompletionRequest::new(
            LLMType::GPT4,
            vec![LLMClientMessage::user(self.user_message(&request))],
            0.7,
            None,
        );

        let llm_properties = LLMProperties::new(
            LLMType::GPT4,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("your-api-key-here".to_owned())),
        );

        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        let response = self
            .llm_client
            .stream_completion(
                llm_properties.api_key().clone(),
                llm_request,
                llm_properties.provider().clone(),
                vec![("root_id".to_owned(), request.root_request_id.clone())]
                    .into_iter()
                    .collect(),
                sender,
            )
            .await
            .map_err(ToolError::LLMClientError)?;

        Ok(ToolOutput::code_summarization(CodeSummarizationResponse {
            summary: response,
        }))
    }
}"#
        .to_string(),
        r#"Step 8: Handle the New ToolType in Relevant Code
Files to Update:

Any match statements that handle ToolType or ToolInput.
For example, in tool/invoke.rs or wherever tools are dispatched.
Example:

match tool_input.tool_type() {
    // ... existing matches ...
    ToolType::CodeSummarization => {
        let tool = SummaryClient::new(llm_client.clone());
        tool.invoke(tool_input).await
    }
    // ... other matches ...
}"#
        .to_string(),
        r#"Step 9: Update Dependencies
File: Cargo.toml

Ensure that llm_client and any other required crates are included.
If SummaryClient introduces new dependencies, add them accordingly.
Example:

[dependencies]
async-trait = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
llm_client = { path = "../path_to_llm_client" }
# ... other dependencies ..."#
            .to_string(),
        r#"Step 10: Add Unit Tests
File: tests/tool_code_summarization.rs (new file)

Write unit tests for SummaryClient:

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic::tool::output::ToolOutput;

    #[tokio::test]
    async fn test_code_summarization() {
        let llm_client = Arc::new(LLMBroker::new());
        let summary_client = SummaryClient::new(llm_client);

        let request = CodeSummarizationRequest {
            code: "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
            root_request_id: "test-root-id".to_string(),
        };

        let input = ToolInput::CodeSummarization(request);
        let output = summary_client.invoke(input).await.unwrap();

        if let ToolOutput::CodeSummarization(response) = output {
            assert!(!response.summary.is_empty());
            println!("Summary: {}", response.summary);
        } else {
            panic!("Expected CodeSummarization output");
        }
    }
}"#
        .to_string(),
    ]
    .iter()
    .enumerate()
    .map(|(index, description)| {
        PlanStep::new(
            description.to_owned(),
            index,
            vec![],
            UserContext::new(vec![], vec![], None, vec![]),
        )
    })
    .collect::<Vec<_>>();

    let mut plan = Plan::new(initial_context, user_query, &steps);

    // make md called plan.md

    // so we just want to use search and replace, right?

    // not even, we just want to invoke the tool.
}
