use std::{path::PathBuf, sync::Arc};

use fancy_regex::Regex;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{
        AnthropicAPIKey, FireworksAPIKey, GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys,
        OpenAIProvider,
    },
};
use quick_xml::de::from_str;
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
            code_edit::{models::broker::CodeEditBroker, types::CodeEditingPartialRequest},
            input::ToolInputPartial,
            lsp::{
                file_diagnostics::WorkspaceDiagnosticsPartial, list_files::ListFilesInput,
                open_file::OpenFileRequestPartial, search_file::SearchFileContentInputPartial,
            },
            r#type::ToolType,
            session::{
                ask_followup_question::AskFollowupQuestionsRequest,
                attempt_completion::AttemptCompletionClientRequest,
                tool_use_agent::{ToolUseAgent, ToolUseAgentInput},
            },
            terminal::terminal::TerminalInputPartial,
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
    let editor_url = "http://localhost:42430".to_owned();
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("".to_owned()));
    let anthropic_llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys.clone(),
    );
    let _llama_70b_properties = LLMProperties::new(
        LLMType::Llama3_1_70bInstruct,
        LLMProvider::FireworksAI,
        LLMProviderAPIKeys::FireworksAI(FireworksAPIKey::new(
            "s8Y7yIXdL0lMeHHgvbZXS77oGtBAHAsfsLviL2AKnzuGpg1n".to_owned(),
        )),
    );
    let _google_ai_studio_api_keys =
        LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new("".to_owned()));

    let llm_client = Arc::new(
        LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
            .await
            .expect("to initialize properly"),
    );
    let editor_parsing = Arc::new(EditorParsing::default());
    let symbol_broker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));
    let tool_broker = Arc::new(ToolBroker::new(
        llm_client.clone(),
        Arc::new(CodeEditBroker::new()),
        symbol_broker.clone(),
        Arc::new(TSLanguageParsing::init()),
        // for our testing workflow we want to apply the edits directly
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::Gpt4O,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("".to_owned())),
        ), // LLMProperties::new(
           //     LLMType::GeminiPro,
           //     LLMProvider::GoogleAIStudio,
           //     LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
           //         "".to_owned(),
           //     )),
           // ),
    ));

    let llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("".to_owned())),
    );

    let tools_to_use = vec![
        ToolType::ListFiles,
        ToolType::SearchFileContentWithRegex,
        ToolType::OpenFile,
        ToolType::CodeEditing,
        ToolType::LSPDiagnostics,
        ToolType::AskFollowupQuestions,
        ToolType::AttemptCompletion,
    ];
    let tool_description = tools_to_use
        .into_iter()
        .filter_map(|tool_to_use| tool_broker.get_tool_description(&tool_to_use))
        .collect::<Vec<_>>();

    let mut tool_input = ToolUseAgentInput::new(
        vec![],
        tool_description,
        "Whats happening with ToolInput?".to_owned(),
        llm_properties,
        "/Users/skcd/scratch/sidecar".to_owned(),
        "darwin".to_owned(),
        "zsh".to_owned(),
    );
    loop {
        let tool_use_agent = ToolUseAgent::new(llm_client.clone());

        let response = tool_use_agent.invoke(tool_input.clone()).await;
        if response.is_err() {
            return;
        }
        let response = response.expect("is_err above to work");
        let parsed_response = parse_out_tool_input(&response);

        // okay now that we have the right thing we want to keep running this as a loop
        // and see what comes out of it
        match parsed_response {
            None => {
                // this implies failure case that we were not able to parse the tool output
                // for now lets break over here
                break;
            }
            Some(tool_input_partial) => match tool_input_partial {
                ToolInputPartial::AskFollowupQuestions(followup_question) => {
                    println!("Ask followup question: {}", followup_question.question());
                }
                ToolInputPartial::AttemptCompletion(_attempt_completion) => {
                    break;
                }
                ToolInputPartial::CodeEditing(code_editing) => {
                    println!("Code editing: {}", code_editing.fs_file_path());
                }
                ToolInputPartial::LSPDiagnostics(diagnostics) => {}
                ToolInputPartial::ListFiles(list_files) => {
                    println!("list files: {}", list_files.directory_path())
                }
                ToolInputPartial::OpenFile(open_file) => {}
                ToolInputPartial::SearchFileContentWithRegex(search_filex) => {}
                ToolInputPartial::TerminalCommand(terminal_command) => {}
            },
        }
    }
}

fn parse_out_tool_input(input: &str) -> Option<ToolInputPartial> {
    let tags = vec![
        "thinking",
        "search_files",
        "code_edit_input",
        "list_files",
        "read_file",
        "get_diagnostics",
        "execute_command",
        "attempt_completion",
        "ask_followup_question",
    ];

    // Build the regex pattern to match any of the tags
    let tags_pattern = tags.join("|");
    let pattern = format!(
        r"(?s)<({tags_pattern})>(.*?)</\1>",
        tags_pattern = tags_pattern
    );

    let re = Regex::new(&pattern).unwrap();
    for cap in re.captures_iter(&input) {
        let capture = cap.expect("to work");
        let tag_name = &capture[1];
        let content = &capture[2];

        // Skip the <thinking> block
        if tag_name == "thinking" {
            continue;
        }

        // Step 2: Map tag to enum variant
        match tag_name {
            "search_files" => {
                // Step 3: Parse the XML content
                let xml_content = format!("<root>{}</root>", content);
                let parsed: SearchFileContentInputPartial = from_str(&xml_content).unwrap();

                // Step 4: Construct the enum variant
                return Some(ToolInputPartial::SearchFileContentWithRegex(parsed));
            }
            "code_edit_input" => {
                // Step 3: Parse the XML content
                let xml_content = format!("<root>{}</root>", content);
                let parsed: CodeEditingPartialRequest = from_str(&xml_content).unwrap();

                // Step 4: Construct the enum variant
                return Some(ToolInputPartial::CodeEditing(parsed));
            }
            "list_files" => {
                // Step 3: Parse the XML content
                let xml_content = format!("<root>{}</root>", content);
                let parsed: ListFilesInput = from_str(&xml_content).unwrap();

                // Step 4: Construct the enum variant
                return Some(ToolInputPartial::ListFiles(parsed));
            }
            "read_file" => {
                // Step 3: Parse the XML content
                let xml_content = format!("<root>{}</root>", content);
                let parsed: OpenFileRequestPartial = from_str(&xml_content).unwrap();

                // Step 4: Construct the enum variant
                return Some(ToolInputPartial::OpenFile(parsed));
            }
            "get_diagnostics" => {
                // Step 3: Parse the XML content
                let xml_content = format!("<root>{}</root>", content);

                return Some(ToolInputPartial::LSPDiagnostics(
                    WorkspaceDiagnosticsPartial::new(),
                ));
            }
            "execute_command" => {
                let xml_content = format!("<root>{}</root>", content);
                let parsed: TerminalInputPartial = from_str(&xml_content).unwrap();
                return Some(ToolInputPartial::TerminalCommand(parsed));
            }
            "attempt_completion" => {
                let xml_content = format!("<root>{}</root>", content);
                let parsed: AttemptCompletionClientRequest = from_str(&xml_content).unwrap();
                return Some(ToolInputPartial::AttemptCompletion(parsed));
            }
            "ask_followup_question" => {
                let xml_content = format!("<root>{}</root>", content);
                let parsed: AskFollowupQuestionsRequest = from_str(&xml_content).unwrap();
                return Some(ToolInputPartial::AskFollowupQuestions(parsed));
            }
            _ => {}
        }
    }

    None
}
