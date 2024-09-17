//! Grabs the outline node for the range

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

    let message_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new("".to_owned(), "".to_owned()),
        sender.clone(),
        editor_url.to_owned(),
        tokio_util::sync::CancellationToken::new(),
    );

    let tool_box = Arc::new(ToolBox::new(tool_broker, symbol_broker, editor_parsing));

    // our outline node fetching is going wonky because of the decorators
    // present on top of classes in rust
    let _range = Range::new(Position::new(27, 0, 0), Position::new(40, 1, 0));
    let fs_file_path = "/Users/skcd/scratch/sidecar/llm_client/src/provider.rs";
    let file_open_response = tool_box
        .file_open(fs_file_path.to_owned(), message_properties.clone())
        .await
        .expect("to work");
    let _ = tool_box
        .force_add_document(
            fs_file_path,
            file_open_response.contents_ref(),
            file_open_response.language(),
        )
        .await;
    let outline_nodes = tool_box
        .get_outline_nodes_grouped(fs_file_path)
        .await
        .expect("to be present");
    outline_nodes
        .into_iter()
        .enumerate()
        .for_each(|(idx, outline_node)| {
            let prompt_data = outline_node.get_outline_for_prompt();
            println!("<idx {}>", idx);
            println!("{}", prompt_data);
            println!("</idx>");
        });
    // let request_id = "something";
    // let result = tool_box
    //     .get_outline_node_for_range(&range, &fs_file_path, request_id)
    //     .await;
    // assert!(result.is_ok());
}
