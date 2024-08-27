//! Contains the handler for agnetic requests and how they work

use super::types::json as json_result;
use axum::response::{sse, IntoResponse, Sse};
use axum::{extract::Query as axumQuery, Extension, Json};
use futures::{stream, StreamExt};
use llm_client::provider::GoogleAIStudioKey;
use llm_client::{
    clients::types::LLMType,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::agentic::symbol::events::input::SymbolEventRequestId;
use crate::agentic::symbol::events::message_event::SymbolEventMessageProperties;
use crate::agentic::symbol::helpers::SymbolFollowupBFS;
use crate::agentic::symbol::identifier::SymbolIdentifier;
use crate::agentic::symbol::tool_properties::ToolProperties;
use crate::agentic::symbol::toolbox::helpers::SymbolChangeSet;
use crate::agentic::tool::broker::ToolBrokerConfiguration;
use crate::{
    agentic::{
        symbol::{
            events::input::SymbolInputEvent, identifier::LLMProperties, manager::SymbolManager,
        },
        tool::{broker::ToolBroker, code_edit::models::broker::CodeEditBroker},
    },
    application::application::Application,
    user_context::types::UserContext,
};

use super::types::ApiResponse;
use super::{model_selection::LLMClientConfig, types::Result};

#[derive(Debug, Clone)]
pub struct ProbeRequestTracker {
    pub running_requests: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl ProbeRequestTracker {
    pub fn new() -> Self {
        Self {
            running_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn track_new_request(&self, request_id: &str, join_handle: JoinHandle<()>) {
        let mut running_requests = self.running_requests.lock().await;
        running_requests.insert(request_id.to_owned(), join_handle);
    }

    async fn cancel_request(&self, request_id: &str) {
        let mut running_requests = self.running_requests.lock().await;
        if let Some(response) = running_requests.get_mut(request_id) {
            // we abort the running requests
            response.abort();
        }
    }
}

/// Contains all the data which we will need to trigger the edits
#[derive(Clone)]
struct AnchoredEditingMetadata {
    message_properties: SymbolEventMessageProperties,
    // These are the symbols where we are focussed on right now in the selection
    anchored_symbols: Vec<(SymbolIdentifier, Vec<String>)>,
    // the context provided by the user
    user_context: Option<String>,
    // we also want to store the original content of the files which were mentioned
    // before we started editing
    previous_file_content: HashMap<String, String>,
}

impl AnchoredEditingMetadata {
    pub fn new(
        message_properties: SymbolEventMessageProperties,
        anchored_symbols: Vec<(SymbolIdentifier, Vec<String>)>,
        user_context: Option<String>,
        previous_file_content: HashMap<String, String>,
    ) -> Self {
        Self {
            message_properties,
            anchored_symbols,
            user_context,
            previous_file_content,
        }
    }
}

pub struct AnchoredEditingTracker {
    running_requests_properties: Arc<Mutex<HashMap<String, AnchoredEditingMetadata>>>,
    running_requests: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl AnchoredEditingTracker {
    pub fn new() -> Self {
        Self {
            running_requests_properties: Arc::new(Mutex::new(HashMap::new())),
            running_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn get_properties(&self, request_id: &str) -> Option<AnchoredEditingMetadata> {
        let running_requests = self.running_requests_properties.lock().await;
        running_requests.get(request_id).map(|data| data.clone())
    }

    async fn track_new_request(
        &self,
        request_id: &str,
        join_handle: JoinHandle<()>,
        editing_metadata: AnchoredEditingMetadata,
    ) {
        {
            let mut running_requests = self.running_requests.lock().await;
            running_requests.insert(request_id.to_owned(), join_handle);
        }
        {
            let mut running_request_properties = self.running_requests_properties.lock().await;
            running_request_properties.insert(request_id.to_owned(), editing_metadata);
        }
    }

    pub async fn override_running_request(&self, request_id: &str, join_handle: JoinHandle<()>) {
        {
            let mut running_requests = self.running_requests.lock().await;
            running_requests.insert(request_id.to_owned(), join_handle);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeRequestActiveWindow {
    file_path: String,
    file_content: String,
    language: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeRequest {
    request_id: String,
    editor_url: String,
    model_config: LLMClientConfig,
    user_context: UserContext,
    query: String,
    active_window_data: Option<ProbeRequestActiveWindow>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeStopRequest {
    request_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeStopResponse {
    done: bool,
}

pub async fn probe_request_stop(
    Extension(app): Extension<Application>,
    Json(ProbeStopRequest { request_id }): Json<ProbeStopRequest>,
) -> Result<impl IntoResponse> {
    println!("webserver::probe_request_stop");
    let probe_request_tracker = app.probe_request_tracker.clone();
    let _ = probe_request_tracker.cancel_request(&request_id).await;
    Ok(Json(ProbeStopResponse { done: true }))
}

pub async fn probe_request(
    Extension(app): Extension<Application>,
    Json(ProbeRequest {
        request_id,
        editor_url,
        model_config,
        mut user_context,
        query,
        active_window_data,
    }): Json<ProbeRequest>,
) -> Result<impl IntoResponse> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let probe_request_tracker = app.probe_request_tracker.clone();
    if let Some(active_window_data) = active_window_data {
        user_context = user_context.update_file_content_map(
            active_window_data.file_path,
            active_window_data.file_content,
            active_window_data.language,
        );
    }
    let provider_keys = model_config
        .provider_for_slow_model()
        .map(|provider| provider.clone())
        .ok_or(anyhow::anyhow!("missing provider for slow model"))?;
    let _provider_type = provider_keys.provider_type();
    let event_message_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new(request_id.to_owned(), request_id.to_owned()),
        sender.clone(),
        editor_url,
    );

    let symbol_manager = app.symbol_manager.clone();

    // spawn a background thread to keep polling the probe_request future
    let join_handle = tokio::spawn(async move {
        let _ = symbol_manager
            .probe_request_from_user_context(query, user_context, event_message_properties.clone())
            .await;
    });

    let _ = probe_request_tracker
        .track_new_request(&request_id, join_handle)
        .await;

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
    gcloud_access_token: String,
    swe_bench_id: String,
}

pub async fn swe_bench(
    axumQuery(SWEBenchRequest {
        git_dname,
        problem_statement,
        editor_url,
        test_endpoint,
        repo_map_file,
        gcloud_access_token: _glcoud_access_token,
        swe_bench_id,
    }): axumQuery<SWEBenchRequest>,
    Extension(app): Extension<Application>,
) -> Result<impl IntoResponse> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let tool_broker = Arc::new(ToolBroker::new(
        app.llm_broker.clone(),
        Arc::new(CodeEditBroker::new()),
        app.symbol_tracker.clone(),
        app.language_parsing.clone(),
        // for swe-bench tests we do not care about tracking edits
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::GeminiPro,
            LLMProvider::GoogleAIStudio,
            LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
            )),
        ),
    ));
    let user_context = UserContext::new(vec![], vec![], None, vec![git_dname]);
    let model = LLMType::ClaudeSonnet;
    let provider_type = LLMProvider::Anthropic;
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
    let symbol_manager = SymbolManager::new(
        tool_broker,
        app.symbol_tracker.clone(),
        app.editor_parsing.clone(),
        LLMProperties::new(
            model.clone(),
            provider_type.clone(),
            anthropic_api_keys.clone(),
        ),
    );

    let message_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new(swe_bench_id.to_owned(), swe_bench_id.to_owned()),
        sender.clone(),
        editor_url.to_owned(),
    );

    println!("we are getting a hit at this endpoint");

    // Now we send the original request over here and then await on the sender like
    // before
    tokio::spawn(async move {
        let _ = symbol_manager
            .initial_request(
                SymbolInputEvent::new(
                    user_context,
                    model,
                    provider_type,
                    anthropic_api_keys,
                    problem_statement,
                    "web_server_input".to_owned(),
                    "web_server_input".to_owned(),
                    Some(test_endpoint),
                    repo_map_file,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                    None,
                    None,
                    false,
                    sender,
                )
                .set_swe_bench_id(swe_bench_id),
                message_properties,
            )
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeSculptingWarmup {
    request_id: String,
    file_paths: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeSculptingWarmupResponse {
    done: bool,
}

impl ApiResponse for CodeSculptingWarmupResponse {}

pub async fn code_sculpting_warmup(
    Extension(app): Extension<Application>,
    Json(CodeSculptingWarmup {
        request_id,
        file_paths,
    }): Json<CodeSculptingWarmup>,
) -> Result<impl IntoResponse> {
    println!(
        "webserver::code_sculpting_warmup::request_id({})",
        &request_id
    );
    let _ = app.tool_box.warmup_context(file_paths, request_id).await;
    Ok(json_result(CodeSculptingWarmupResponse { done: true }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeSculptingHeal {
    request_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeSculptingHealResponse {
    done: bool,
}

impl ApiResponse for CodeSculptingHealResponse {}

pub async fn code_sculpting_heal(
    Extension(app): Extension<Application>,
    Json(CodeSculptingHeal { request_id }): Json<CodeSculptingHeal>,
) -> Result<impl IntoResponse> {
    println!(
        "webserver::code_sculpting_heal::request_id({})",
        &request_id
    );
    let anchor_properties;
    {
        let anchor_tracker = app.anchored_request_tracker.clone();
        anchor_properties = anchor_tracker.get_properties(&request_id).await;
    }
    println!(
        "code_sculpting::heal::request_id({})::properties_present({})",
        request_id,
        anchor_properties.is_some()
    );
    if anchor_properties.is_none() {
        Ok(json_result(CodeSculptingHealResponse { done: false }))
    } else {
        let anchor_properties = anchor_properties.expect("is_none to hold");
        let older_file_content_map = anchor_properties.previous_file_content;
        let file_paths = anchor_properties
            .anchored_symbols
            .iter()
            .filter_map(|symbol| symbol.0.fs_file_path())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let message_properties = anchor_properties.message_properties.clone();

        // Now grab the symbols which have changed
        let cloned_tools = app.tool_box.clone();
        let symbol_change_set: HashMap<String, SymbolChangeSet> =
            stream::iter(file_paths.into_iter().map(|file_path| {
                let older_file_content = older_file_content_map
                    .get(&file_path)
                    .map(|content| content.to_owned());
                (
                    file_path,
                    cloned_tools.clone(),
                    older_file_content,
                    message_properties.clone(),
                )
            }))
            .map(
                |(fs_file_path, tools, older_file_content, message_properties)| async move {
                    if let Some(older_content) = older_file_content {
                        let file_content = tools
                            .file_open(fs_file_path.to_owned(), message_properties)
                            .await
                            .ok();
                        if let Some(new_content) = file_content {
                            tools
                                .get_symbol_change_set(
                                    &fs_file_path,
                                    &older_content,
                                    new_content.contents_ref(),
                                )
                                .await
                                .ok()
                                .map(|symbol_change_set| (fs_file_path, symbol_change_set))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                },
            )
            .buffer_unordered(10)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(|s| s)
            .collect::<HashMap<_, _>>();

        let changed_symbols = anchor_properties
            .anchored_symbols
            .into_iter()
            .filter_map(|(symbol_identifier, _)| {
                let fs_file_path = symbol_identifier.fs_file_path();
                if fs_file_path.is_none() {
                    return None;
                }
                let fs_file_path = fs_file_path.clone().expect("is_none to hold");
                let changed_symbols_in_file = symbol_change_set.get(&fs_file_path);
                if let Some(changed_symbols_in_file) = changed_symbols_in_file {
                    let symbol_changes = changed_symbols_in_file
                        .changes()
                        .into_iter()
                        .filter(|changed_symbol| {
                            changed_symbol.symbol_identifier().symbol_name()
                                == symbol_identifier.symbol_name()
                        })
                        .map(|changed_symbol| changed_symbol.clone())
                        .collect::<Vec<_>>();
                    Some(symbol_changes)
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        let followup_bfs_request = changed_symbols
            .into_iter()
            .map(|changes| {
                let symbol_identifier = changes.symbol_identifier().clone();
                let symbol_identifier_ref = &symbol_identifier;
                changes
                    .remove_changes()
                    .into_iter()
                    .map(|symbol_to_edit| {
                        SymbolFollowupBFS::new(
                            symbol_to_edit.0,
                            symbol_identifier_ref.clone(),
                            symbol_to_edit.1,
                            symbol_to_edit.2,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        let hub_sender = app.symbol_manager.hub_sender();
        let cloned_tools = app.tool_box.clone();
        let _join_handle = tokio::spawn(async move {
            let _ = cloned_tools
                .check_for_followups_bfs(
                    followup_bfs_request,
                    hub_sender,
                    message_properties,
                    &ToolProperties::new(),
                )
                .await;
        });
        Ok(json_result(CodeSculptingHealResponse { done: true }))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeSculptingRequest {
    request_id: String,
    instruction: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeSculptingResponse {
    done: bool,
}

impl ApiResponse for CodeSculptingResponse {}

pub async fn code_sculpting(
    Extension(app): Extension<Application>,
    Json(CodeSculptingRequest {
        request_id,
        instruction,
    }): Json<CodeSculptingRequest>,
) -> Result<impl IntoResponse> {
    let anchor_properties;
    {
        let anchor_tracker = app.anchored_request_tracker.clone();
        anchor_properties = anchor_tracker.get_properties(&request_id).await;
    }
    println!(
        "code_sculpting::instruction({})::properties_present({})",
        instruction,
        anchor_properties.is_some()
    );
    if anchor_properties.is_none() {
        Ok(json_result(CodeSculptingResponse { done: false }))
    } else {
        let anchor_properties = anchor_properties.expect("is_none to hold");
        let symbol_manager = app.symbol_manager.clone();
        let join_handle = tokio::spawn(async move {
            let anchored_symbols = anchor_properties.anchored_symbols;
            let user_context = anchor_properties.user_context;
            let message_properties = anchor_properties.message_properties;
            let _ = symbol_manager
                .anchor_edits(
                    instruction,
                    anchored_symbols,
                    user_context,
                    message_properties,
                )
                .await;
        });
        {
            let anchor_tracker = app.anchored_request_tracker.clone();
            let _ = anchor_tracker
                .override_running_request(&request_id, join_handle)
                .await;
        }
        Ok(json_result(CodeSculptingResponse { done: true }))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgenticCodeEditing {
    user_query: String,
    editor_url: String,
    request_id: String,
    user_context: UserContext,
    active_window_data: Option<ProbeRequestActiveWindow>,
    root_directory: String,
    codebase_search: bool,
    // If we are editing based on an anchor position
    anchor_editing: bool,
}

pub async fn code_editing(
    Extension(app): Extension<Application>,
    Json(AgenticCodeEditing {
        user_query,
        editor_url,
        request_id,
        mut user_context,
        active_window_data,
        root_directory,
        codebase_search,
        // this is not properly hooked up yet, we need to figure out
        // how to handle this better on the editor side, right now our proxy
        // is having a selection item in the user_context
        mut anchor_editing,
    }): Json<AgenticCodeEditing>,
) -> Result<impl IntoResponse> {
    println!("webserver::code_editing_start::request_id({})", &request_id);
    let edit_request_tracker = app.probe_request_tracker.clone();
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    if let Some(active_window_data) = active_window_data {
        user_context = user_context.update_file_content_map(
            active_window_data.file_path,
            active_window_data.file_content,
            active_window_data.language,
        );
    }

    println!("{:?}", &user_context);

    let message_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new(request_id.to_owned(), request_id.to_owned()),
        sender.clone(),
        editor_url,
    );

    anchor_editing = anchor_editing || user_context.is_anchored_editing();

    println!(
        "webserver::code_editing_flow::endpoint_hit::anchor_editing({})",
        anchor_editing
    );
    if anchor_editing {
        println!(
            "webserver::code_editing_flow::anchor_editing::({})",
            anchor_editing
        );
        let symbols_to_anchor = app
            .tool_box
            .symbols_to_anchor(&user_context, message_properties.clone())
            .await
            .unwrap_or_default();
        println!(
            "webserver::code_editing_flow::anchor_symbols::({})",
            symbols_to_anchor
                .iter()
                .map(|(symbol, _)| symbol.symbol_name())
                .collect::<Vec<_>>()
                .join(",")
        );
        if !symbols_to_anchor.is_empty() {
            // if we do not have any symbols to anchor on, then we are screwed over here
            // we want to send the edit request directly over here cutting through
            // the initial request parts
            let user_provided_context = user_context.to_context_string().await.ok();
            let possibly_changed_files = symbols_to_anchor
                .iter()
                .filter_map(|(symbol_identifer, _)| symbol_identifer.fs_file_path())
                .collect::<Vec<_>>();
            let cloned_tools = app.tool_box.clone();
            let file_contents =
                stream::iter(possibly_changed_files.into_iter().map(|file_path| {
                    (file_path, cloned_tools.clone(), message_properties.clone())
                }))
                .map(|(fs_file_path, tools, message_properties)| async move {
                    let file_open_response = tools
                        .file_open(fs_file_path.to_owned(), message_properties)
                        .await;
                    file_open_response
                        .ok()
                        .map(|response| (fs_file_path, response.contents()))
                })
                .buffer_unordered(100)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .filter_map(|s| s)
                .collect::<HashMap<_, _>>();
            let editing_metadata = AnchoredEditingMetadata::new(
                message_properties.clone(),
                symbols_to_anchor,
                user_provided_context.clone(),
                file_contents,
            );
            let anchored_symbols = app
                .tool_box
                .symbols_to_anchor(&user_context, message_properties.clone())
                .await
                .unwrap_or_default();
            let symbol_manager = app.symbol_manager.clone();
            let join_handle = tokio::spawn(async move {
                let _ = symbol_manager
                    .anchor_edits(
                        user_query,
                        anchored_symbols,
                        user_provided_context,
                        message_properties,
                    )
                    .await;
            });
            let _ = app
                .anchored_request_tracker
                .track_new_request(&request_id, join_handle, editing_metadata)
                .await;
            let properties_present = app
                .anchored_request_tracker
                .get_properties(&request_id)
                .await;
            println!(
                "webserver::anchored_edits::request_id({})::properties_present({})",
                &request_id,
                properties_present.is_some()
            );
        }
    } else {
        println!("webserver::code_editing_flow::agnetic_editing");
        let edit_request_id = request_id.clone(); // Clone request_id before creating the closure
                                                  // Now we send the original request over here and then await on the sender like
                                                  // before

        let symbol_manager = app.symbol_manager.clone();
        let join_handle = tokio::spawn(async move {
            let _ = symbol_manager
            .initial_request(
                SymbolInputEvent::new(
                    user_context,
                    LLMType::ClaudeSonnet,
                    LLMProvider::Anthropic,
                    LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned())),
                    user_query,
                    edit_request_id.to_owned(),
                    edit_request_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    true,
                    Some(root_directory),
                    None,
                    codebase_search, // big search
                    sender,
                ),
                message_properties,
            )
            .await;
        });
        let _ = edit_request_tracker
            .track_new_request(&request_id, join_handle)
            .await;
    }

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnchorSessionStart {
    request_id: String,
    user_context: UserContext,
    root_directory: String,
    editor_url: String,
    active_window_data: Option<ProbeRequestActiveWindow>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnchorSessionStartResponse {
    done: bool,
}

impl ApiResponse for AnchorSessionStartResponse {}

pub async fn anchor_session_start(
    Extension(app): Extension<Application>,
    Json(AnchorSessionStart {
        request_id,
        mut user_context,
        root_directory,
        editor_url,
        active_window_data,
    }): Json<AnchorSessionStart>,
) -> Result<impl IntoResponse> {
    println!(
        "webserver::anchor_session_start::request_id({})",
        &request_id
    );

    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    if let Some(active_window_data) = active_window_data {
        user_context = user_context.update_file_content_map(
            active_window_data.file_path,
            active_window_data.file_content,
            active_window_data.language,
        );
    }

    println!("{:?}", &user_context);

    let message_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new(request_id.to_owned(), request_id.to_owned()),
        sender.clone(),
        editor_url,
    );

    println!(
        "webserver::agentic::anchor_session_start::user_context::variables:\n{}",
        user_context
            .variables
            .iter()
            .map(|var| var.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let symbol_to_anchor = app
        .tool_box
        .symbols_to_anchor(&user_context, message_properties.clone())
        .await
        .unwrap_or_default();

    Ok(json_result(AnchorSessionStartResponse { done: false }))

    // now we iterate through each variable

    // find outline nodes for each

    // go to references on them.
}
