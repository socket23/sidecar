//! Contains a script code which  can be used to test out swe bench
//! and how its working

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
        tool::{broker::ToolBroker, code_edit::models::broker::CodeEditBroker},
    },
    application::logging::tracing::tracing_subscribe_default,
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    user_context::types::UserContext,
};
use tokio::process::Command;

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscribe_default();
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
    let user_context = UserContext::new(
        vec![],
        vec![],
        None,
        vec!["/Users/skcd/scratch/sidecar/sidecar/".to_owned()],
    );
    let gemini_pro_keys = LLMProviderAPIKeys::GeminiPro(GeminiProAPIKey::new("ya29.a0AXooCguiRZP_3G8vUxvkKgrEfcTyGu-xdqdv5SyXsgvWKuaxJSjjTTRH7_cvzsYrOqyyZ_P7-gQFw_L1VRsl1xITfFsvTbVJLsaYUqVGBwKNG4d8obg6OQctm36QxeWwTGYNvke10k_oMW1ygkhIzjIsogk_d_PnBfecn8TubmkaCgYKAeMSARESFQHGX2MiUhp9vFKvNq1Lp7CMO-x2pA0178".to_owned(), "anton-390822".to_owned()));
    // this is the current running debuggable editor
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
    ));
    let _gemini_llm_properties = LLMProperties::new(
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
        anthropic_llm_properties.clone(),
        user_context.clone(),
    );

    tokio::spawn(async move {
        let mut child = Command::new("/Users/skcd/scratch/swe_bench/swe_bench/bin/python3")
            .arg("/Users/skcd/scratch/swe_bench/run_editor_for_instance.py")
            .arg("django__django-11179")
            .arg("/var/folders/bq/1dbw218x1zq3r3c5_gqxgdgr0000gn/T/tmp02jxzkk5")
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("to work");
        // busy waiting on the child
        let _ = child.wait().await;
    });

    let problem_statement = r#"delete() on instances of models without any dependencies doesn't clear PKs.

Description

Deleting any model with no dependencies not updates the PK on the model. It should be set to None after .delete() call.

See Django.db.models.deletion:276-281. Should update the model line 280."#.to_owned();
    let initial_request = SymbolInputEvent::new(
        UserContext::new(
            vec![],
            vec![],
            None,
            vec!["/var/folders/bq/1dbw218x1zq3r3c5_gqxgdgr0000gn/T/tmp02jxzkk5".to_owned()],
        ),
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys,
        problem_statement,
        Some("http://localhost:6897/run_tests".to_owned()),
        Some("/var/folders/bq/1dbw218x1zq3r3c5_gqxgdgr0000gn/T/tmpu88w4cw3".to_owned()),
        Some("ya29.a0AXooCgt_BiTCT9QBPN-xpsT3QJj6h6rgAFFiZJV_8NOyaK2KjxE8eI9OgVTE83rhJoh_oqEBGBWbYWiDR1YHw_lQJ7dtSWRZRPsngHOy5iVzrpAtxY9cuBDa5aPg_ZNKbR4rcnttYDWvociuzlcv8m-HOPhZIyJoMhtFJdZKJGNLaCgYKAbESARESFQHGX2MiZF_klhi8iTdvsK7l4TicOQ0179".to_owned()),
        Some("django__django-11179".to_owned()),
    );
    let mut probe_task = Box::pin(symbol_manager.initial_request(initial_request));

    loop {
        tokio::select! {
            event = receiver.recv() => {
                if let Some(_event) = event {
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
