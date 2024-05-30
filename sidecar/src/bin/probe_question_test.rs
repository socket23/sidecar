//! Here we are going to test if the probe query is working as we would expect
//! it to
use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, GeminiProAPIKey, LLMProvider, LLMProviderAPIKeys},
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
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
    let user_context = UserContext::new(
        vec![],
        vec![],
        None,
        vec!["/Users/skcd/scratch/sidecar/sidecar/".to_owned()],
    );
    let gemini_pro_keys = LLMProviderAPIKeys::GeminiPro(GeminiProAPIKey::new("ya29.a0AXooCguiRZP_3G8vUxvkKgrEfcTyGu-xdqdv5SyXsgvWKuaxJSjjTTRH7_cvzsYrOqyyZ_P7-gQFw_L1VRsl1xITfFsvTbVJLsaYUqVGBwKNG4d8obg6OQctm36QxeWwTGYNvke10k_oMW1ygkhIzjIsogk_d_PnBfecn8TubmkaCgYKAeMSARESFQHGX2MiUhp9vFKvNq1Lp7CMO-x2pA0178".to_owned(), "anton-390822".to_owned()));
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
    let gemini_llm_properties = LLMProperties::new(
        LLMType::GeminiProFlash,
        LLMProvider::GeminiPro,
        gemini_pro_keys.clone(),
    );
    let anthropic_llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys.clone(),
    );
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let symbol_manager = SymbolManager::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing,
        editor_url.to_owned(),
        sender,
        // This is where we are setting the LLM properties
        gemini_llm_properties.clone(),
        user_context.clone(),
    );

    // let agent_symbol_identifier = SymbolIdentifier::with_file_path(
    //     "Agent",
    //     "/Users/skcd/scratch/sidecar/sidecar/src/agent/types.rs",
    // );
    // let agent_request = "Where are we sending the request to the LLM client?".to_owned();
    // let symbol_identifier = SymbolIdentifier::with_file_path(
    //     "agent_router",
    //     "/Users/skcd/scratch/sidecar/sidecar/src/bin/webserver.rs",
    // );
    // let symbol_request = "how id model configuration passed to the llm client in agent? start from here cause this is the how the webserver handles the request coming from elsewhere. I want to focus on whats the data structure and where it is used to exchange this information with the llm client".to_owned();
    let symbol_probing_tool_use = SymbolIdentifier::with_file_path(
        "main",
        "/Users/skcd/scratch/sidecar/sidecar/src/bin/probe_question_test.rs",
    );
    let symbol_probing_request = "What are the tools which are used we initiate a probe request on a symbol (assume the complete code workflow), exaplain in detail to me the ToolTypes which is being used and how it is being used.";
    let probe_request = SymbolToProbeRequest::new(
        symbol_probing_tool_use.clone(),
        symbol_probing_request.to_owned(),
        symbol_probing_request.to_owned(),
        vec![],
    );
    let probe_request = SymbolEventRequest::probe_request(symbol_probing_tool_use, probe_request);
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
