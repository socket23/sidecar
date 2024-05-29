use std::{path::PathBuf, sync::Arc};

use llm_client::{broker::LLMBroker, config::LLMBrokerConfiguration};
use sidecar::{
    agentic::{
        symbol::{identifier::Snippet, tool_box::ToolBox},
        tool::{broker::ToolBroker, code_edit::models::broker::CodeEditBroker},
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
    let editor_url = "http://localhost:42423".to_owned();
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
    ));

    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

    let tool_box = Arc::new(ToolBox::new(
        tool_broker,
        symbol_broker,
        editor_parsing,
        editor_url,
        sender,
    ));

    let range = Range::new(Position::new(139, 0, 0), Position::new(157, 0, 0));
    let fs_file_path =
        "/Users/skcd/scratch/sidecar/sidecar/src/webserver/agent_stream.rs".to_owned();
    let line_content = "    mut agent: Agent,".to_owned();
    let symbol_to_search = "Agent".to_owned();
    // This is what I have to debug
    let response = tool_box
        .go_to_definition_using_symbol(&range, &fs_file_path, &line_content, &symbol_to_search)
        .await;
    println!("{:?}", response);
}
