use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys},
};
use sidecar::{
    agentic::{
        symbol::{
            events::{
                input::SymbolEventRequestId, message_event::SymbolEventMessageProperties,
                probe::SubSymbolToProbe,
            },
            identifier::LLMProperties,
            tool_box::ToolBox,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
        },
    },
    chunking::{
        editor_parsing::EditorParsing,
        languages::TSLanguageParsing,
        text_document::{Position, Range},
    },
    inline_completion::symbols_tracker::SymbolTrackerInline,
};

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    let editor_url = "http://localhost:42424".to_owned();
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
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::GeminiPro,
            LLMProvider::GoogleAIStudio,
            LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
            )),
        ),
    ));

    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

    // fill this
    let access_token = String::from("");
    let event_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new("".to_owned(), "".to_owned()),
        sender.clone(),
        editor_url.to_owned(),
        tokio_util::sync::CancellationToken::new(),
        access_token,
    );

    let tool_box = Arc::new(ToolBox::new(tool_broker, symbol_broker, editor_parsing));

    let file_path = "/Users/skcd/scratch/sidecar/sidecar/src/agent/search.rs".to_owned();
    let sub_symbol_probe = SubSymbolToProbe::new(
        "code_search_hybrid".to_owned(),
        Range::new(Position::new(320, 4, 0), Position::new(465, 5, 0)),
        file_path.to_owned(),
        "something".to_owned(),
        false,
    );

    let parent_symbol_name = "Agent";

    // let file_open_response = tool_box
    //     .file_open(file_path.to_owned(), "testing")
    //     .await
    //     .expect("to work");

    // let _ = tool_box
    //     .force_add_document(
    //         &file_path,
    //         file_open_response.contents_ref(),
    //         file_open_response.language(),
    //     )
    //     .await;

    let result = tool_box
        .find_sub_symbol_to_probe_with_name(parent_symbol_name, &sub_symbol_probe, event_properties)
        .await;
    println!("{:?}", result);
}
