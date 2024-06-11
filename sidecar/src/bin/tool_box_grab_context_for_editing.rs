use std::{path::PathBuf, sync::Arc};

use llm_client::{broker::LLMBroker, config::LLMBrokerConfiguration};
use sidecar::{
    agentic::{
        symbol::{events::edit::SymbolToEdit, tool_box::ToolBox},
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
    let editor_url = "http://localhost:6897".to_owned();
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
        None,
    ));

    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

    let tool_box = Arc::new(ToolBox::new(
        tool_broker,
        symbol_broker,
        editor_parsing,
        editor_url,
        sender,
    ));

    let file_path = "django/test/utils.py".to_owned();

    let symbol_to_edit = SymbolToEdit::new(
        "setup_databases".to_owned(),
        Range::new(Position::new(158, 0, 4471), Position::new(205, 20, 6494)),
        file_path.to_owned(),
        vec![],
        false,
    );

    let file_open_response = tool_box
        .file_open(file_path.to_owned(), "testing")
        .await
        .expect("to always work");
    let _ = tool_box
        .force_add_document(
            &file_path,
            file_open_response.contents_ref(),
            file_open_response.language(),
        )
        .await;
    let result = tool_box.find_sub_symbol_to_edit(&symbol_to_edit).await;
    println!("{:?}", result);
}
