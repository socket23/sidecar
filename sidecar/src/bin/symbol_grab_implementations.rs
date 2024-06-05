//! We are going to test out how the grab implementations part is working
//! over here as a E2E script

use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, LLMProvider},
};
use sidecar::{
    agentic::{
        symbol::{
            identifier::{LLMProperties, MechaCodeSymbolThinking, Snippet, SymbolIdentifier},
            tool_box::ToolBox,
            types::Symbol,
        },
        tool::{broker::ToolBroker, code_edit::models::broker::CodeEditBroker},
    },
    chunking::{
        editor_parsing::EditorParsing,
        languages::TSLanguageParsing,
        text_document::{Position, Range},
        types::OutlineNodeContent,
    },
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
    let fs_file_path = "/Users/skcd/scratch/sidecar/sidecar/src/agent/types.rs".to_owned();
    let placeholder_range = Range::new(Position::new(0, 0, 0), Position::new(0, 0, 0));
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

    let mecha_code_symbol_thinking = MechaCodeSymbolThinking::new(
        "Agent".to_owned(),
        vec![],
        false,
        fs_file_path.to_owned(),
        Some(Snippet::new(
            "Agent".to_owned(),
            placeholder_range.clone(),
            fs_file_path.to_owned(),
            "".to_owned(),
            OutlineNodeContent::new(
                "Agent".to_owned(),
                placeholder_range.clone(),
                sidecar::chunking::types::OutlineNodeType::Class,
                "".to_owned(),
                fs_file_path.to_owned(),
                placeholder_range.clone(),
                placeholder_range.clone(),
            ),
        )),
        vec![],
        UserContext::new(vec![], vec![], None, vec![]),
    );

    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let (ui_sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

    let symbol = Symbol::new(
        SymbolIdentifier::with_file_path("Agent", &fs_file_path),
        mecha_code_symbol_thinking,
        sender,
        tool_box,
        LLMProperties::new(
            LLMType::ClaudeOpus,
            LLMProvider::Anthropic,
            llm_client::provider::LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(
                "".to_owned(),
            )),
        ),
        ui_sender,
        "".to_owned(),
    )
    .await
    .expect("to work");
}
