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
            events::{input::SymbolEventRequestId, message_event::SymbolEventMessageProperties},
            identifier::LLMProperties,
            tool_box::ToolBox,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
        },
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
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
    let editor_url = "http://localhost:42450".to_owned();
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

    let fs_file_path = "/Users/skcd/test_repo/sidecar/llm_client/src/provider.rs".to_owned();
    let file_open_request = tool_box
        .file_open(fs_file_path.to_owned(), event_properties.clone())
        .await
        .expect("to work");
    let _ = tool_box
        .force_add_document(
            &fs_file_path,
            file_open_request.contents_ref(),
            file_open_request.language(),
        )
        .await;
    let outline_nodes = tool_box
        .get_outline_nodes_grouped(&fs_file_path)
        .await
        .expect("to work");

    outline_nodes.into_iter().for_each(|outline_node| {
        let outline_node_name = outline_node.name();
        let trait_implementation = outline_node.content().has_trait_implementation();
        println!("{}::{:?}", outline_node_name, trait_implementation);
    });

    // let _ = Range::new(Position::new(139, 0, 0), Position::new(157, 0, 0));
    // let _ = "post(sidecar::webserver::agent::followup_chat),".to_owned();
    // let symbol_to_search = "LLMProvider".to_owned();
    // // This is what I have to debug
    // let snippet = tool_box
    //     .find_snippet_for_symbol(&fs_file_path, &symbol_to_search, "")
    //     .await;
    // println!("{:?}", &snippet);
}
