use async_openai::Client;
use async_openai::{config::OpenAIConfig, types::CreateCompletionRequestArgs};
use futures::StreamExt;
use reqwest::Client as ReqwestClient;
use sidecar::llm::clients::ollama::OllamaClient;
use sidecar::llm::clients::types::{LLMClient, LLMClientCompletionRequest};
use sidecar::reporting::posthog::client::{posthog_client, PosthogClient};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct OllamaResponse {
    model: String,
    response: String,
    done: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = OllamaClient::new();
    let request = LLMClientCompletionRequest::new(
        "mixtral:latest".to_owned(),
        "<s>[INST] Can you write me a python function which adds 2 numbers [/INST]".to_owned(),
        0.7,
        None,
    );
    let response = client.completion(request).await;
    Ok(())
}
