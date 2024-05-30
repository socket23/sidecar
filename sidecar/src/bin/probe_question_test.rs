//! Here we are going to test if the probe query is working as we would expect
//! it to
use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};
use sidecar::{
    agentic::{
        symbol::{
            events::{input::SymbolInputEvent, probe::SymbolToProbeRequest},
            identifier::{LLMProperties, SymbolIdentifier},
            manager::SymbolManager,
            types::SymbolEventRequest,
        },
        tool::{
            broker::ToolBroker,
            code_edit::models::broker::CodeEditBroker,
            code_symbol::important::{
                CodeSymbolImportantResponse, CodeSymbolWithSteps, CodeSymbolWithThinking,
            },
        },
    },
    application::logging::tracing::tracing_subscribe_default,
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
    tracing_subscribe_default();
    let current_query =
        "Where are we sending the request to the LLM clients? from the agent".to_owned();
    let anthropic_api_key = "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".to_owned();
    let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key));
    let user_context = UserContext::new(
        vec![],
        vec![],
        None,
        vec!["/Users/skcd/scratch/sidecar/sidecar/".to_owned()],
    );
    // this is the current running debuggable editor
    let editor_url = "http://localhost:59293".to_owned();
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
    let llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        api_key.clone(),
    );
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let symbol_manager = SymbolManager::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing,
        editor_url.to_owned(),
        sender,
        llm_properties,
        user_context.clone(),
    );
    let symbol_input = SymbolInputEvent::new(
        user_context,
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        api_key,
        current_query.to_owned(),
    );

    // let agent_symbol_identifier = SymbolIdentifier::with_file_path(
    //     "Agent",
    //     "/Users/skcd/scratch/sidecar/sidecar/src/agent/types.rs",
    // );
    // let agent_request = "Where are we sending the request to the LLM client?".to_owned();
    let symbol_identifier = SymbolIdentifier::with_file_path(
        "agent_router",
        "/Users/skcd/scratch/sidecar/sidecar/src/bin/webserver.rs",
    );
    let symbol_request = "how id model configuration passed to the llm client in agent? start from here cause this is the how the webserver handles the request coming from elsewhere. I want to focus on whats the data structure and where it is used to exchange this information with the llm client".to_owned();
    let probe_request = SymbolToProbeRequest::new(
        symbol_identifier.clone(),
        symbol_request.to_owned(),
        symbol_request.to_owned(),
        vec![],
    );
    let probe_request = SymbolEventRequest::probe_request(symbol_identifier, probe_request);
    let mut probe_task = Box::pin(symbol_manager.probe_request(probe_request));

    loop {
        tokio::select! {
            event = receiver.recv() => {
                if let Some(event) = event {
                    // info!("event: {:?}", event);
                } else {
                    break; // Receiver closed, exit the loop
                }
            }
            _ = &mut probe_task => {
                // probe_task completed, you can handle it here if needed
            }
        }
    }
}
