use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, CodeStoryLLMTypes, LLMProvider, LLMProviderAPIKeys},
};
use sidecar::{
    agentic::{
        mecha::{
            basic::{MechaBasic, MechaEvent, SymbolLocking},
            events::input::MechaInputEvent,
        },
        tool::{broker::ToolBroker, code_edit::models::broker::CodeEditBroker},
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

// TODO: we need more symbol intelligence somehow to be able to do things

#[tokio::main]
async fn main() {
    let anthropic_api_key = "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".to_owned();
    let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key));
    let user_context = UserContext::new(
        vec![],
        vec![],
        None,
        vec!["/Users/skcd/scratch/sidecar/llm_client".to_owned()],
    );
    let editor_url = "".to_owned();
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
    let mecha_input = MechaInputEvent::new(
        user_context,
        LLMType::ClaudeHaiku,
        LLMProvider::Anthropic,
        api_key,
        "I want to create a new groq provider".to_owned(),
    );

    let symbol_locking = SymbolLocking::new();

    // now create the mecha
    let mut mecha = MechaBasic::new(
        tool_broker,
        symbol_broker,
        editor_parsing,
        symbol_locking.clone(),
        editor_url,
    );

    // Lets execute the first event
    let response = mecha.iterate(MechaEvent::InitialRequest(mecha_input)).await;
    println!("hello world");
}
