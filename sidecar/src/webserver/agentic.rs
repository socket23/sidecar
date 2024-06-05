//! Contains the handler for agnetic requests and how they work

use axum::response::{sse, IntoResponse, Sse};
use axum::{extract::Query as axumQuery, Extension, Json};
use futures::StreamExt;
use llm_client::{
    clients::types::LLMType,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};
use serde_json::json;
use std::{sync::Arc, time::Duration};

use crate::{
    agentic::{
        symbol::{
            events::{input::SymbolInputEvent, probe::SymbolToProbeRequest},
            identifier::{LLMProperties, SymbolIdentifier},
            manager::SymbolManager,
            types::SymbolEventRequest,
        },
        tool::{broker::ToolBroker, code_edit::models::broker::CodeEditBroker},
    },
    application::application::Application,
    user_context::types::UserContext,
};

use super::{model_selection::LLMClientConfig, types::Result};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeRequest {
    editor_url: String,
    model_config: LLMClientConfig,
    user_context: UserContext,
    symbol_identifier: SymbolIdentifier,
    query: String,
}

pub async fn probe_request(
    Extension(app): Extension<Application>,
    Json(ProbeRequest {
        editor_url,
        model_config,
        user_context,
        symbol_identifier,
        query,
    }): Json<ProbeRequest>,
) -> Result<impl IntoResponse> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let tool_broker = Arc::new(ToolBroker::new(
        app.llm_broker.clone(),
        Arc::new(CodeEditBroker::new()),
        app.symbol_tracker.clone(),
        app.language_parsing.clone(),
    ));
    let provider_keys = model_config
        .provider_for_slow_model()
        .map(|provider| provider.clone())
        .ok_or(anyhow::anyhow!("missing provider for slow model"))?;
    let provider_type = provider_keys.provider_type();
    let symbol_manager = SymbolManager::new(
        tool_broker,
        app.symbol_tracker.clone(),
        app.editor_parsing.clone(),
        editor_url.to_owned(),
        sender,
        LLMProperties::new(model_config.slow_model, provider_type, provider_keys),
        user_context,
    );
    let probe_request = SymbolToProbeRequest::new(
        symbol_identifier.clone(),
        query.to_owned(),
        query.to_owned(),
        vec![],
    );
    let probe_request = SymbolEventRequest::probe_request(symbol_identifier, probe_request);
    // spawn a background thread to keep polling the probe_request future
    tokio::spawn(async move {
        let _ = symbol_manager.probe_request(probe_request).await;
    });

    // Now we want to poll the future of the probe request we are sending
    // along with the ui events so we can return the channel properly
    // how do go about doing that?
    let event_stream = Sse::new(
        tokio_stream::wrappers::UnboundedReceiverStream::new(receiver).map(|event| {
            sse::Event::default()
                .json_data(event)
                .map_err(anyhow::Error::new)
        }),
    );

    // return the stream as a SSE event stream over here
    Ok(event_stream.keep_alive(
        sse::KeepAlive::new()
            .interval(Duration::from_secs(3))
            .event(
                sse::Event::default()
                    .json_data(json!({
                        "keep_alive": "alive"
                    }))
                    .expect("json to not fail in keep alive"),
            ),
    ))
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SWEBenchRequest {
    git_dname: String,
    problem_statement: String,
    editor_url: String,
    test_endpoint: String,
    // This is the file path with the repo map present in it
    repo_map_file: Option<String>,
}

pub async fn swe_bench(
    axumQuery(SWEBenchRequest {
        git_dname,
        problem_statement,
        editor_url,
        test_endpoint,
        repo_map_file,
    }): axumQuery<SWEBenchRequest>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let tool_broker = Arc::new(ToolBroker::new(
        app.llm_broker.clone(),
        Arc::new(CodeEditBroker::new()),
        app.symbol_tracker.clone(),
        app.language_parsing.clone(),
    ));
    let user_context = UserContext::new(vec![], vec![], None, vec![git_dname]);
    let model = LLMType::ClaudeSonnet;
    let provider_type = LLMProvider::Anthropic;
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
    let symbol_manager = SymbolManager::new(
        tool_broker,
        app.symbol_tracker.clone(),
        app.editor_parsing.clone(),
        editor_url.to_owned(),
        sender,
        LLMProperties::new(
            model.clone(),
            provider_type.clone(),
            anthropic_api_keys.clone(),
        ),
        user_context.clone(),
    );

    println!("we are getting a hit at this endpoint");

    // Now we send the original request over here and then await on the sender like
    // before
    tokio::spawn(async move {
        let _ = symbol_manager
            .initial_request(SymbolInputEvent::new(
                user_context,
                model,
                provider_type,
                anthropic_api_keys,
                problem_statement,
                Some(test_endpoint),
                repo_map_file,
            ))
            .await;
    });
    let event_stream = Sse::new(
        tokio_stream::wrappers::UnboundedReceiverStream::new(receiver).map(|event| {
            sse::Event::default()
                .json_data(event)
                .map_err(anyhow::Error::new)
        }),
    );

    // return the stream as a SSE event stream over here
    Ok(event_stream.keep_alive(
        sse::KeepAlive::new()
            .interval(Duration::from_secs(3))
            .event(
                sse::Event::default()
                    .json_data(json!({
                        "keep_alive": "alive"
                    }))
                    .expect("json to not fail in keep alive"),
            ),
    ))
}
