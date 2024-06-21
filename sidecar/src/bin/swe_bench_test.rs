//! Contains a script code which  can be used to test out swe bench
//! and how its working

use std::{env, path::PathBuf, process::Stdio, sync::Arc};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{
        AnthropicAPIKey, GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys, OpenAIProvider,
    },
};
use serde_json::json;
use sidecar::{
    agentic::{
        symbol::{
            events::input::SymbolInputEvent, identifier::LLMProperties, manager::SymbolManager,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
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

async fn get_diff_patch(git_dname: &str) -> String {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(git_dname)
        .arg("--no-pager") // Add this line to disable the pager
        .arg("diff")
        .stdout(Stdio::piped())
        .spawn()
        .expect("to work");
    let _ = child.wait().await;
    let mut stdout = child.stdout.take().expect("Failed to get stdout");
    let mut output = Vec::new();
    stdout.read_to_end(&mut output).await.expect("to work");

    let output_string = String::from_utf8_lossy(&output);
    output_string.to_string()
}

// Over here we are going to pass a json which has all the important information which we need
// to solve a single swe-bench-test, passed via an env variable (cause I am lazy)

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SWEBenchInput {
    instance_id: String,
    gemini_api_key: String,
    repo_map_fs_path: String,
    repo_path: String,
    problem_statement: String,
}

#[tokio::main]
async fn main() {
    let content = env::var("swe_bench_input_path").expect("to always be present");
    let input: SWEBenchInput = serde_json::from_slice(
        &tokio::fs::read(&content)
            .await
            .expect("file reading to always work"),
    )
    .expect("to work");
    tracing_subscribe_default();
    let instance_id = input.instance_id.to_owned();
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
    let gemini_llm_properties = LLMProperties::new(
        LLMType::GeminiPro,
        LLMProvider::GoogleAIStudio,
        LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
            "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
        )),
    );
    let anthropic_llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys.clone(),
    );
    let code_editing_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys.clone(),
    );
    let gpt4o_config = LLMProperties::new(
        LLMType::Gpt4O,
        LLMProvider::OpenAI,
        LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
            "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".to_owned(),
        )),
    );
    // this is the current running debuggable editor
    let user_context = UserContext::new(
        vec![],
        vec![],
        None,
        vec!["/Users/skcd/scratch/sidecar/sidecar/".to_owned()],
    );

    // editor running
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
        Some(ToolBrokerConfiguration::new(Some(code_editing_properties))),
        LLMProperties::new(
            LLMType::GeminiPro,
            LLMProvider::GoogleAIStudio,
            LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
            )),
        ),
    ));
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

    // I should create symlinks for these so its easier to query as well :|
    let folder_path = input.repo_path.to_owned();
    // let folder_path = "/var/folders/bq/1dbw218x1zq3r3c5_gqxgdgr0000gn/T/tmp9khfwaj0".to_owned();
    let repo_map_fs_path = input.repo_map_fs_path.to_owned();
    // let repo_map_fs_path =
    //     "/var/folders/bq/1dbw218x1zq3r3c5_gqxgdgr0000gn/T/tmpb0s1ot0p".to_owned();
    let problem_statement = input.problem_statement.to_owned();
    let initial_request = SymbolInputEvent::new(
        UserContext::new(vec![], vec![], None, vec![folder_path.to_owned()]),
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys,
        problem_statement,
        instance_id.to_owned(),
        Some("http://localhost:6897/run_tests".to_owned()),
        Some(repo_map_fs_path.to_owned()),
        Some(input.gemini_api_key.to_owned()),
        Some(instance_id.to_owned()),
        Some(folder_path.to_owned()),
        Some(gpt4o_config),
        Some(gemini_llm_properties),
    );
    let mut initial_request_task = Box::pin(symbol_manager.initial_request(initial_request));

    loop {
        tokio::select! {
            event = receiver.recv() => {
                if let Some(_event) = event {
                    // info!("event: {:?}", event);
                } else {
                    break; // Receiver closed, exit the loop
                }
            }
            result = &mut initial_request_task => {
                match result {
                    Ok(_) => {
                        // The task completed successfully
                        // Handle the result if needed
                    }
                    Err(e) => {
                        // An error occurred while running the task
                        eprintln!("Error in initial_request_task: {}", e);
                        // Handle the error appropriately (e.g., log, retry, or exit)
                    }
                }
            }
        }
    }

    // Over here we should write out the json file so we can run evaluation on it
    let prediction_output = "/Users/skcd/scratch/swe_bench/predictions/full---gpt-4o/".to_owned()
        + &instance_id
        + ".jsonl";
    // Now we write out the json object required for the predictions to work
    let prediction_json = json!({
        "instance_id": instance_id.to_owned(),
        "model_name_or_path": "codestory-mixed".to_owned(),
        "model_patch": get_diff_patch(&folder_path).await,
    });

    let _ = tokio::fs::write(
        prediction_output,
        serde_json::to_string(&prediction_json).expect("serde to not fail"),
    )
    .await;
}
