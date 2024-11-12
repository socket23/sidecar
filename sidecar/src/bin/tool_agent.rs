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
                edit::SymbolToEdit, input::SymbolEventRequestId,
                message_event::SymbolEventMessageProperties,
            },
            identifier::{LLMProperties, SymbolIdentifier},
            manager::SymbolManager,
            tool_box::ToolBox,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
            input::{ToolInput, ToolInputPartial},
            lsp::{open_file::OpenFileRequest, search_file::SearchFileContentInput},
            r#type::{Tool, ToolType},
            session::service::SessionService,
            terminal::terminal::TerminalInput,
        },
    },
    chunking::{
        editor_parsing::EditorParsing,
        languages::TSLanguageParsing,
        text_document::{Position, Range},
    },
    inline_completion::symbols_tracker::SymbolTrackerInline,
    repo::types::RepoRef,
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
    let editor_url = "http://localhost:42427".to_owned();
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
        ),
    ));

    let tool_box = Arc::new(ToolBox::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing.clone(),
    ));

    let session_id = uuid::Uuid::new_v4().to_string();
    let mut session_path = default_index_dir().join("session");
    // check if the plan_storage_path_exists
    if tokio::fs::metadata(&session_path).await.is_err() {
        tokio::fs::create_dir(&session_path)
            .await
            .expect("directory creation to not fail");
    }
    session_path = session_path.join(session_id.to_owned());

    let symbol_tracker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));

    let symbol_manager = Arc::new(SymbolManager::new(
        tool_broker.clone(),
        symbol_tracker.clone(),
        editor_parsing.clone(),
        LLMProperties::new(
            LLMType::ClaudeSonnet,
            LLMProvider::Anthropic,
            LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("".to_owned())),
        ),
    ));

    let session_service = SessionService::new(tool_box.clone(), symbol_manager);

    let tools_to_use = vec![
        ToolType::ListFiles,
        ToolType::SearchFileContentWithRegex,
        ToolType::OpenFile,
        ToolType::CodeEditing,
        ToolType::LSPDiagnostics,
        // disable for testing
        // ToolType::AskFollowupQuestions,
        ToolType::AttemptCompletion,
    ];
    let repo_ref = RepoRef::local("/Users/skcd/scratch/sidecar").expect("to work");
    let mut session = session_service.create_new_session_with_tools(
        &session_id,
        vec![],
        repo_ref.clone(),
        session_path.to_string_lossy().to_string(),
        tools_to_use.to_vec(),
    );

    let mut exchange_id = 0;
    let initial_query = "Can you find all structs which implement the Tool trait.".to_owned();
    session = session.human_message(
        exchange_id.to_string(),
        initial_query,
        UserContext::default(),
        vec![],
        repo_ref.clone(),
    );

    loop {
        let parent_exchange_id = exchange_id;
        exchange_id = exchange_id + 1;
        let (tool_to_use, session_new) = session
            .clone()
            .get_tool_to_use(
                tool_box.clone(),
                llm_client.clone(),
                "/Users/skcd/scratch/sidecar".to_owned(),
                "darwin".to_owned(),
                "zsh".to_owned(),
                exchange_id.to_string(),
                parent_exchange_id.to_string(),
                anthropic_llm_properties.clone(),
            )
            .await;
        session = session_new;
        // okay now that we have the right thing we want to keep running this as a loop
        // and see what comes out of it
        match tool_to_use {
            None => {
                // this implies failure case that we were not able to parse the tool output
                // for now lets break over here
                break;
            }
            Some(tool_input_partial) => match tool_input_partial {
                ToolInputPartial::AskFollowupQuestions(followup_question) => {
                    println!("Ask followup question: {}", followup_question.question());
                    let input = ToolInput::AskFollowupQuestions(followup_question);
                    let response = tool_broker.invoke(input).await;
                    println!("response: {:?}", response);
                }
                ToolInputPartial::AttemptCompletion(attempt_completion) => {
                    println!("LLM reached a stop condition");
                    println!("{:?}", &attempt_completion);
                    break;
                }
                ToolInputPartial::CodeEditing(code_editing) => {
                    let fs_file_path = code_editing.fs_file_path().to_owned();
                    println!("Code editing: {}", fs_file_path);
                    let (sender, mut _receiver) = tokio::sync::mpsc::unbounded_channel();

                    let message_properties = SymbolEventMessageProperties::new(
                        SymbolEventRequestId::new(request_id_str.clone(), request_id_str.clone()),
                        sender.clone(),
                        editor_url.to_owned(),
                        tokio_util::sync::CancellationToken::new(),
                        anthropic_llm_properties.clone(),
                    );

                    let file_contents = tool_box
                        .file_open(fs_file_path.to_owned(), message_properties.clone())
                        .await
                        .expect("to work")
                        .contents();

                    let instruction = code_editing.instruction().to_owned();

                    let default_range = Range::new(Position::new(0, 0, 0), Position::new(0, 0, 0));

                    let symbol_to_edit = SymbolToEdit::new(
                        "".to_owned(),
                        default_range,
                        fs_file_path.to_owned(),
                        vec![instruction.clone()],
                        false,
                        false, // is_new
                        false,
                        "".to_owned(),
                        None,
                        false,
                        None,
                        false,
                        None,
                        vec![], // previous_user_queries
                        None,
                    );

                    let symbol_identifier = SymbolIdentifier::new_symbol("");

                    let response = tool_box
                        .code_editing_with_search_and_replace(
                            &symbol_to_edit,
                            &fs_file_path,
                            &file_contents,
                            &default_range,
                            "".to_owned(),
                            instruction.clone(),
                            &symbol_identifier,
                            None,
                            None,
                            message_properties,
                        )
                        .await
                        .expect("to work"); // big expectations

                    println!("response: {:?}", response);
                }
                ToolInputPartial::LSPDiagnostics(diagnostics) => {
                    println!("LSP diagnostics: {:?}", diagnostics);
                }
                ToolInputPartial::ListFiles(list_files) => {
                    println!("list files: {}", list_files.directory_path());
                    let input = ToolInput::ListFiles(list_files);
                    let response = tool_broker.invoke(input).await;
                    let list_files_output = response
                        .expect("to work")
                        .get_list_files_directory()
                        .expect("to work");
                    let response = list_files_output
                        .files()
                        .into_iter()
                        .map(|file_path| file_path.to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    exchange_id = exchange_id + 1;
                    session = session.human_message(
                        exchange_id.to_string(),
                        response.to_owned(),
                        UserContext::default(),
                        vec![],
                        repo_ref.clone(),
                    );
                    println!("response: {:?}", response);
                }
                ToolInputPartial::OpenFile(open_file) => {
                    println!("open file: {}", open_file.fs_file_path());
                    let open_file_path = open_file.fs_file_path().to_owned();
                    let request = OpenFileRequest::new(open_file_path, editor_url.clone());
                    let input = ToolInput::OpenFile(request);
                    let response = tool_broker
                        .invoke(input)
                        .await
                        .expect("to work")
                        .get_file_open_response()
                        .expect("to work")
                        .to_string();
                    exchange_id = exchange_id + 1;
                    session = session.human_message(
                        exchange_id.to_string(),
                        response.clone(),
                        UserContext::default(),
                        vec![],
                        repo_ref.clone(),
                    );
                    println!("response: {:?}", response);
                }
                ToolInputPartial::SearchFileContentWithRegex(search_file) => {
                    println!("search file: {}", search_file.directory_path());
                    let request = SearchFileContentInput::new(
                        search_file.directory_path().to_owned(),
                        search_file.regex_pattern().to_owned(),
                        search_file.file_pattern().map(|s| s.to_owned()),
                        editor_url.clone(),
                    );
                    let input = ToolInput::SearchFileContentWithRegex(request);
                    let tool_response = tool_broker.invoke(input).await.expect("to work");
                    let response = tool_response
                        .get_search_file_content_with_regex()
                        .expect("to work");
                    let response = response.response();
                    exchange_id = exchange_id + 1;
                    session = session.human_message(
                        exchange_id.to_string(),
                        response.to_owned(),
                        UserContext::default(),
                        vec![],
                        repo_ref.clone(),
                    );
                    println!("response: {:?}", response);
                }
                ToolInputPartial::TerminalCommand(terminal_command) => {
                    println!("terminal command: {}", terminal_command.command());
                    let command = terminal_command.command().to_owned();
                    let request = TerminalInput::new(command, editor_url.clone());
                    let input = ToolInput::TerminalCommand(request);
                    let tool_output = tool_broker.invoke(input).await;
                    let output = tool_output
                        .expect("to work")
                        .terminal_command()
                        .expect("to work")
                        .output()
                        .to_owned();
                    exchange_id = exchange_id + 1;
                    session = session.human_message(
                        exchange_id.to_string(),
                        output.to_owned(),
                        UserContext::default(),
                        vec![],
                        repo_ref.clone(),
                    );
                    println!("response: {:?}", output);
                }
            },
        }
    }
}
