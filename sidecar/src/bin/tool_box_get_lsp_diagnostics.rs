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
    let editor_url = "http://localhost:42430".to_owned();
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

    let tool_box = Arc::new(ToolBox::new(tool_broker, symbol_broker, editor_parsing));

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
    let fs_file_path =
        "/Users/skcd/test_repo/sidecar/sidecar/src/agentic/tool/plan/plan_step.rs".to_owned();
    let references = tool_box
        .broken_references_for_lsp_diagnostics(&fs_file_path, event_properties.clone())
        .await;
    println!("{:?}", references);
    let file_diagnostics = tool_box
        .get_lsp_diagnostics_for_files(
            vec![fs_file_path.to_owned()],
            event_properties.clone(),
            false,
        )
        .await;
    println!("{:?}", file_diagnostics);

    // let fs_file_path = "/Users/skcd/scratch/sidecar/llm_client/src/broker.rs".to_owned();
    // let lsp_range = Range::new(Position::new(51, 0, 0), Position::new(89, 0, 0));
    // let response = tool_box
    //     .get_lsp_diagnostics(&fs_file_path, &lsp_range, event_properties)
    //     .await;
    // println!("{:?}", &response);
}
