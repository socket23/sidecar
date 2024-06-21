use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys},
};
use sidecar::{
    agentic::{
        symbol::{identifier::LLMProperties, tool_box::ToolBox},
        tool::{
            broker::ToolBroker,
            code_edit::models::broker::CodeEditBroker,
            code_symbol::important::{
                CodeSymbolImportantResponse, CodeSymbolWithSteps, CodeSymbolWithThinking,
            },
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
        None,
        LLMProperties::new(
            LLMType::GeminiPro,
            LLMProvider::GoogleAIStudio,
            LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
            )),
        ),
    ));

    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

    let tool_box = Arc::new(ToolBox::new(
        tool_broker,
        symbol_broker,
        editor_parsing,
        editor_url,
        sender,
    ));

    let important_symbols = CodeSymbolImportantResponse::new(
        vec![
            CodeSymbolWithThinking::new(
                "agent_router".to_owned(),
                "".to_owned(),
                "src/bin/webserver.rs".to_owned(),
            ),
            CodeSymbolWithThinking::new(
                "_print_sinc".to_owned(),
                "".to_owned(),
                "src/bin/webserver.rs".to_owned(),
            ),
        ],
        vec![
            CodeSymbolWithSteps::new(
                "CCodePrinter".to_owned(),
                vec![],
                false,
                "sympy/printing/ccode.py".to_owned(),
            ),
            CodeSymbolWithSteps::new(
                "_print_sinc".to_owned(),
                vec![],
                true,
                "sympy/printing/ccode.py".to_owned(),
            ),
        ],
    );
    let user_context = UserContext::new(vec![], vec![], None, vec![]);
    let response = tool_box
        .important_symbols(&important_symbols, user_context, "")
        .await;
    println!("{:?}", response);
}
