use llm_client::{
    clients::{
        fireworks::FireworksAIClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::{FireworksAPIKey, LLMProviderAPIKeys},
};

#[tokio::main]
async fn main() {
    let system_message = r#"You are a search engine which makes no mistakes while retriving important context for a user-query.
You will be given context which the user has selected in <user_context> and you have to retrive the "code symbols" which are important for answering to the user query.
- The user might have selected some context manually in the form of <selection> these might be more important
- You will be given files which contains a lot of code, you have to select the "code symbols" which are important
- "code symbols" here referes to the different classes, functions, enums, methods or constants which might be necessary to answer the user query.
- Now you will write a step by step process for making the code edit, this ensures that you lay down the plan before making the change, put this in an xml section called <step_by_step> where each step is in <step_list> section where each section has the name of the symbol on which the operation will happen, if no such symbol exists and you need to create a new one put a <new>true</new> inside the step section and after the symbols
- In your step by step list make sure that the symbols are listed in the order in which we have to go about making the changes
- If we are using absolute paths, make sure to use absolute paths in your reply.
- Strictly follow the reply format which is mentioned to you below, your reply should always start with <reply> tag and end with </reply> tag

Let's focus on getting the "code symbols" which are necessary to satisfy the user query.

As an example, given the following code selection:
<code_selection>
<file_path>
/broker/fill_in_middle.rs
</file_path>
```rust
pub struct FillInMiddleBroker {
    providers: HashMap<LLMType, Box<dyn FillInMiddleFormatter + Send + Sync>>,
}

impl FillInMiddleBroker {
    pub fn new() -> Self {
        let broker = Self {
            providers: HashMap::new(),
        };
        broker
            .add_llm(
                LLMType::CodeLlama13BInstruct,
                Box::new(CodeLlamaFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::CodeLlama7BInstruct,
                Box::new(CodeLlamaFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder1_3BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder6BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder33BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeHaiku,
                Box::new(ClaudeFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeOpus,
                Box::new(ClaudeFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeSonnet,
                Box::new(ClaudeFillInMiddleFormatter::new()),
            )
    }
```
</code_selection>

and the user query is:
<user_query>
I want to add support for the grok llm
</user_query>

Your reply should be, you should strictly follow this format:
<reply>
<symbol_list>
<symbol>
<name>
LLMType
</name>
<file_path>
/broker/fill_in_middle.rs
</file_path>
<thinking>
We need to first check if grok is part of the LLMType enum, this will make sure that the code we produce is never wrong
</thinking>
</symbol>
<symbol>
<name>
FillInMiddleFormatter
</name>
<file_path>
/broker/fill_in_middle.rs
</file_path>
<thinking>
Other LLM's are implementing FillInMiddleFormatter trait, grok will also require support for this, so we need to check how to implement FillInMiddleFormatter trait
</thinking>
</symbol>
<symbol>
<name>
new
</name>
<file_path>
/broker/fill_in_middle.rs
</file_path>
<thinking>
We have to change the new function and add the grok llm after implementing the formatter for grok llm.
</thinking>
</symbol>
</symbol_list>
<step_by_step>
<step_list>
<name>
LLMType
</name>
<file_path>
/broker/fill_in_middle.rs
</file_path>
<step>
We will need to first check the LLMType if it has support for grok or we need to edit it first
</step>
</step_list>
<step_list>
<name>
FillInMiddleFormatter
</name>
<file_path>
/broker/fill_in_middle.rs
</file_path>
<step>
Check the definition of `FillInMiddleFormatter` to see how to implement it
</step>
</step_list>
<step_list
<name>
CodeLlamaFillInMiddleFormatter
</name>
<file_path>
/broker/fill_in_middle.rs
</file_path>
<step>
We can follow the implementation of CodeLlamaFillInMiddleFormatter since we will also have to follow a similar pattern of making changes and adding it to the right places if there are more.
</step>
</step_list>
<step_list>
<name>
GrokFillInMiddleFormatter
</name>
<file_path>
/broker/fill_in_middle.rs
</file_path>
<new>
true
</new>
<step>
Implement the GrokFillInMiddleFormatter following the similar pattern in `CodeLlamaFillInMiddleFormatter`
</step>
</step_list>
</step_by_step>
</reply>

Another example:
<file_path>
/src/bin/webserver.rs
</file_path>
<code_selection>
```rust
fn tree_sitter_router() -> Router {
    use axum::routing::*;
    Router::new()
        .route(
            "/documentation_parsing",
            post(sidecar::webserver::tree_sitter::extract_documentation_strings),
        )
        .route(
            "/diagnostic_parsing",
            post(sidecar::webserver::tree_sitter::extract_diagnostics_range),
        )
        .route(
            "/tree_sitter_valid",
            post(sidecar::webserver::tree_sitter::tree_sitter_node_check),
        )
}

fn file_operations_router() -> Router {
    use axum::routing::*;
    Router::new().route("/edit_file", post(sidecar::webserver::file_edit::file_edit))
}

fn inline_completion() -> Router {
    use axum::routing::*;
    Router::new()
        .route(
            "/inline_completion",
            post(sidecar::webserver::inline_completion::inline_completion),
        )
        .route(
            "/cancel_inline_completion",
            post(sidecar::webserver::inline_completion::cancel_inline_completion),
        )
        .route(
            "/document_open",
            post(sidecar::webserver::inline_completion::inline_document_open),
        )
        .route(
            "/document_content_changed",
            post(sidecar::webserver::inline_completion::inline_completion_file_content_change),
        )
        .route(
            "/get_document_content",
            post(sidecar::webserver::inline_completion::inline_completion_file_content),
        )
        .route(
            "/get_identifier_nodes",
            post(sidecar::webserver::inline_completion::get_identifier_nodes),
        )
        .route(
            "/get_symbol_history",
            post(sidecar::webserver::inline_completion::symbol_history),
        )
}

// TODO(skcd): Figure out why we are passing the context in the suffix and not the prefix

```
</code_selection>

and the user query is:
<user_query>
I want to get the list of most important symbols in inline completions
</user_query>

Your reply should be:
<reply>
<symbol_list>
<symbol>
<name>
inline_completion
</name>
<file_path>
/src/bin/webserver.rs
</file_path>
<thinking>
inline_completion holds all the endpoints for symbols because it also has the `get_symbol_history` endpoint. We have to start adding the endpoint there
</thinking>
</symbol>
<symbol>
<name>
symbol_history
</name>
<file_path>
/src/bin/webserver.rs
</file_path>
<thinking>
I can find more information on how to write the code for the endpoint by following the symbol `symbol_history` in the line: `             post(sidecar::webserver::inline_completion::symbol_history),`
<thinking>
</symbol>
</symbol_list>
<step_by_step>
<step_list>
<name>
symbol_history
</name>
<file_path>
/src/bin/webserver.rs
</file_path>
<thinking>
We need to follow the symbol_history to check the pattern on how we are going to implement the very similar functionality
</thinking>
</step_list>
<step_list>
<name>
inline_completion
</name>
<file_path>
/src/bin/webserver.rs
</file_path>
<thinking>
We have to add the newly created endpoint in inline_completion to add support for the new endpoint which we want to create
</thinking>
</step_list>
</step_by_step>
</reply>"#;
    let user_message = r#"<selection>


<selection_item>
<file>
<file_path>
/Users/zi/codestory/testing/sidecar/sidecar/src/bin/webserver.rs
</file_path>
<content>
```rust
// This is where we will create the default webserver for running the binary
// locally

use anyhow::Result;
use axum::extract::DefaultBodyLimit;
use axum::routing::get;
use axum::Extension;
use clap::Parser;
use sidecar::{
    application::{application::Application, config::configuration::Configuration},
    bg_poll::background_polling::poll_repo_updates,
};
use std::net::SocketAddr;
use tokio::signal;
use tokio::sync::oneshot;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer};
use tracing::{debug, error, info};

pub type Router<S = Application> = axum::Router<S>;

#[tokio::main]
async fn main() -> Result<()> {
    info!("CodeStory 🚀");
    let configuration = Configuration::parse();

    // We get the logging setup first
    debug!("installing logging to local file");
    Application::install_logging(&configuration);

    // Create a oneshot channel
    let (tx, rx) = oneshot::channel();

    // Spawn a task to listen for signals
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("failed to listen for event");
        let _ = tx.send(());
    });

    // We initialize the logging here
    let application = Application::initialize(configuration).await?;
    println!("initialized application");
    debug!("initialized application");

    // Main logic
    tokio::select! {
        // Start the webserver
        _ = run(application) => {
            // Your server logic
        }
        _ = rx => {
            // Signal received, this block will be executed.
            // Drop happens automatically when variables go out of scope.
            debug!("Signal received, cleaning up...");
        }
    }

    Ok(())
}

pub async fn run(application: Application) -> Result<()> {
    let mut joins = tokio::task::JoinSet::new();

    // Start background tasks here
    if application.config.enable_background_polling {
        tokio::spawn(poll_repo_updates(application.clone()));
    }

    joins.spawn(start(application));

    while let Some(result) = joins.join_next().await {
        if let Ok(Err(err)) = result {
            error!(?err, "sidecar failed");
            return Err(err);
        }
    }

    Ok(())
}

// TODO(skcd): Add routes here which can do the following:
// - when a file changes, it should still be logged and tracked
// - when a file is opened, it should be tracked over here too
pub async fn start(app: Application) -> anyhow::Result<()> {
    println!("Port: {}", app.config.port);
    let bind = SocketAddr::new(app.config.host.parse()?, app.config.port);
    let mut api = Router::new()
        .route("/config", get(sidecar::webserver::config::get))
        .route(
            "/reach_the_devs",
            get(sidecar::webserver::config::reach_the_devs),
        )
        .route("/version", get(sidecar::webserver::config::version))
        .nest("/repo", repo_router())
        .nest("/agent", agent_router())
        .nest("/in_editor", in_editor_router())
        .nest("/tree_sitter", tree_sitter_router())
        .nest("/file", file_operations_router())
        .nest("/inline_completion", inline_completion())
        .nest("/agentic", agentic_router());

    api = api.route("/health", get(sidecar::webserver::health::health));

    let api = api
        .layer(Extension(app.clone()))
        .with_state(app.clone())
        .with_state(app.clone())
        .layer(CorsLayer::permissive())
        .layer(CatchPanicLayer::new())
        // I want to set the bytes limit here to 20 MB
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024));

    let router = Router::new().nest("/api", api);

    axum::Server::bind(&bind)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

fn repo_router() -> Router {
    use axum::routing::*;
    Router::new()
        // 127.0.0.1:42424/api/repo/sync?backend=local/{path_absolute}
        .route("/sync", get(sidecar::webserver::repos::sync))
        .route("/status", get(sidecar::webserver::repos::index_status))
        // Gives back the status of the queue
        .route("/queue", get(sidecar::webserver::repos::queue_status))
        // Gives back the repos we know about
        .route("/repo_list", get(sidecar::webserver::repos::repo_status))
}

fn agentic_router() -> Router {
    use axum::routing::*;
    Router::new()
        .route(
            "/probe_request",
            post(sidecar::webserver::agentic::probe_request),
        )
        .route(
            "/probe_request_stop",
            post(sidecar::webserver::agentic::probe_request_stop),
        )
        .route(
            "/code_editing",
            post(sidecar::webserver::agentic::code_editing),
        )
        .route("/swe_bench", get(sidecar::webserver::agentic::swe_bench))
}

fn agent_router() -> Router {
    use axum::routing::*;
    Router::new()
        .route(
            "/search_agent",
            get(sidecar::webserver::agent::search_agent),
        )
        .route(
            "/hybrid_search",
            get(sidecar::webserver::agent::hybrid_search),
        )
        .route("/explain", get(sidecar::webserver::agent::explain))
        .route(
            "/followup_chat",
            post(sidecar::webserver::agent::followup_chat),
        )
}

fn in_editor_router() -> Router {
    use axum::routing::*;
    Router::new().route(
        "/answer",
        post(sidecar::webserver::in_line_agent::reply_to_user),
    )
}

fn tree_sitter_router() -> Router {
    use axum::routing::*;
    Router::new()
        .route(
            "/documentation_parsing",
            post(sidecar::webserver::tree_sitter::extract_documentation_strings),
        )
        .route(
            "/diagnostic_parsing",
            post(sidecar::webserver::tree_sitter::extract_diagnostics_range),
        )
        .route(
            "/tree_sitter_valid",
            post(sidecar::webserver::tree_sitter::tree_sitter_node_check),
        )
        .route(
            "/valid_xml",
            post(sidecar::webserver::tree_sitter::check_valid_xml),
        )
}

fn file_operations_router() -> Router {
    use axum::routing::*;
    Router::new().route("/edit_file", post(sidecar::webserver::file_edit::file_edit))
}

fn inline_completion() -> Router {
    use axum::routing::*;
    Router::new()
        .route(
            "/inline_completion",
            post(sidecar::webserver::inline_completion::inline_completion),
        )
        .route(
            "/cancel_inline_completion",
            post(sidecar::webserver::inline_completion::cancel_inline_completion),
        )
        .route(
            "/document_open",
            post(sidecar::webserver::inline_completion::inline_document_open),
        )
        .route(
            "/document_content_changed",
            post(sidecar::webserver::inline_completion::inline_completion_file_content_change),
        )
        .route(
            "/get_document_content",
            post(sidecar::webserver::inline_completion::inline_completion_file_content),
        )
        .route(
            "/get_identifier_nodes",
            post(sidecar::webserver::inline_completion::get_identifier_nodes),
        )
        .route(
            "/get_symbol_history",
            post(sidecar::webserver::inline_completion::symbol_history),
        )
}

// TODO(skcd): Figure out why we are passing the context in the suffix and not the prefix

```
</content>
</file>
</selection_item>
<selection_item>
<file>
<file_path>
/Users/zi/codestory/testing/sidecar/sidecar/src/webserver/agentic.rs
</file_path>
<content>
```rust
//! Contains the handler for agnetic requests and how they work

use axum::response::{sse, IntoResponse, Sse};
use axum::{extract::Query as axumQuery, Extension, Json};
use futures::StreamExt;
use llm_client::provider::{GoogleAIStudioKey, OpenAIProvider};
use llm_client::{
    clients::types::LLMType,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};
use serde_json::json;
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

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
    let tool_broker = Arc::new(ToolBroker::new(
        app.llm_broker.clone(),
        Arc::new(CodeEditBroker::new()),
        app.symbol_tracker.clone(),
        app.language_parsing.clone(),
        ToolBrokerConfiguration::new(None, false),
        LLMProperties::new(
            LLMType::GeminiPro,
            LLMProvider::CodeStory(Default::default()),
            LLMProviderAPIKeys::CodeStory,
        ),
    ));
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
    let symbol_manager = SymbolManager::new(
        tool_broker,
        app.symbol_tracker.clone(),
        app.editor_parsing.clone(),
        editor_url.to_owned(),
        sender,
        LLMProperties::new(
            LLMType::ClaudeSonnet,
            LLMProvider::CodeStory(Default::default()),
            LLMProviderAPIKeys::CodeStory,
        ),
        // LLMProperties::new(model_config.slow_model, provider_type, provider_keys),
        user_context.clone(),
        request_id.to_owned(),
    );
    // spawn a background thread to keep polling the probe_request future
    let join_handle = tokio::spawn(async move {
        let _ = symbol_manager
            .probe_request_from_user_context(query, user_context)
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
        gcloud_access_token,
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
        editor_url.to_owned(),
        sender,
        LLMProperties::new(
            model.clone(),
            provider_type.clone(),
            anthropic_api_keys.clone(),
        ),
        user_context.clone(),
        swe_bench_id.to_owned(),
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
                    Some(test_endpoint),
                    repo_map_file,
                    Some(gcloud_access_token),
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                    false,
                    None,
                )
                .set_swe_bench_id(swe_bench_id),
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
pub struct AgenticCodeEditing {
    user_query: String,
    editor_url: String,
    request_id: String,
    user_context: UserContext,
    active_window_data: Option<ProbeRequestActiveWindow>,
    root_directory: String,
    codebase_search: bool,
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
    }): Json<AgenticCodeEditing>,
) -> Result<impl IntoResponse> {
    println!("webserver::code_editing_start");
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let edit_request_tracker = app.probe_request_tracker.clone();
    let tool_broker = Arc::new(ToolBroker::new(
        app.llm_broker.clone(),
        Arc::new(CodeEditBroker::new()),
        app.symbol_tracker.clone(),
        app.language_parsing.clone(),
        // do not apply the edits directly
        ToolBrokerConfiguration::new(None, false),
        LLMProperties::new(
            LLMType::Gpt4O,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
                "sk-proj-BLaSMsWvoO6FyNwo9syqT3BlbkFJo3yqCyKAxWXLm4AvePtt".to_owned(),
            )),
        ), // LLMProperties::new(
           //     LLMType::GeminiPro,
           //     LLMProvider::GoogleAIStudio,
           //     LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
           //         "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
           //     )),
           // ),
    ));
    if let Some(active_window_data) = active_window_data {
        user_context = user_context.update_file_content_map(
            active_window_data.file_path,
            active_window_data.file_content,
            active_window_data.language,
        );
    }

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
        request_id.to_owned(),
    );

    println!("webserver::code_editing_flow::endpoint_hit");

    let edit_request_id = request_id.clone(); // Clone request_id before creating the closure
                                              // Now we send the original request over here and then await on the sender like
                                              // before
    let join_handle = tokio::spawn(async move {
        let _ = symbol_manager
            .initial_request(SymbolInputEvent::new(
                user_context,
                model,
                provider_type,
                anthropic_api_keys,
                user_query,
                edit_request_id,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                true,
                codebase_search,
                Some(root_directory),
            ))
            .await;
    });
    let _ = edit_request_tracker
        .track_new_request(&request_id, join_handle)
        .await;

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

```
</content>
</file>
</selection_item>
</selection>
<user_query>
Add support for a new stop_code_editing endpoint and implement it similar to probing stop
</user_query>"#;
    let llm_request = LLMClientCompletionRequest::new(
        LLMType::Llama3_1_70bInstruct,
        vec![
            LLMClientMessage::system(system_message.to_owned()),
            LLMClientMessage::user(user_message.to_owned()),
        ],
        0.2,
        None,
    );
    let client = FireworksAIClient::new();
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = client
        .stream_completion(
            LLMProviderAPIKeys::FireworksAI(FireworksAPIKey::new(
                "s8Y7yIXdL0lMeHHgvbZXS77oGtBAHAsfsLviL2AKnzuGpg1n".to_owned(),
            )),
            llm_request,
            sender,
        )
        .await;
    println!("response:\n{}", response.expect("to work always"));
}
