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
                edit::SymbolToEdit, input::SymbolEventRequestId,
                message_event::SymbolEventMessageProperties,
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

    let event_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new("".to_owned(), "".to_owned()),
        sender.clone(),
        editor_url.to_owned(),
        tokio_util::sync::CancellationToken::new(),
    );

    let tool_box = Arc::new(ToolBox::new(tool_broker, symbol_broker, editor_parsing));
    // This is what I have to debug
    let response = tool_box
        .find_sub_symbol_to_edit_with_name(
            "LLMBroker",
            &SymbolToEdit::new(
                "new".to_owned(),
                Range::new(Position::new(51, 4, 1542), Position::new(86, 5, 3339)),
                "/Users/skcd/scratch/sidecar/llm_client/src/broker.rs".to_owned(),
                vec![],
                false,
                false,
                false,
                "".to_owned(),
                None,
                false,
                None,
                false, // should we disable followups and correctness check
                None,
                vec![],
            ),
            event_properties,
        )
        .await;
    println!("{:?}", response);
}
