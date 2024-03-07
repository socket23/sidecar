use llm_client::{
    clients::{
        anthropic::AnthropicClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMClientRole, LLMType},
    },
    provider::{AnthropicAPIKey, LLMProviderAPIKeys},
};
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() {
    let anthropic_api_key = "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".to_owned();
    let anthropic_client = AnthropicClient::new();
    let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key));
    let request = LLMClientCompletionRequest::new(
        LLMType::ClaudeOpus,
        vec![
            LLMClientMessage::new(LLMClientRole::System, "you are an expert".to_owned()),
            LLMClientMessage::new(LLMClientRole::User, "Can you say 5, 5 times".to_owned()),
        ],
        0.1,
        None,
    )
    .set_max_tokens(100);
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = anthropic_client
        .stream_completion(api_key, request, sender)
        .await;
    println!("{:?}", response);
    // let client = Client::new();
    // let url = "https://api.anthropic.com/v1/messages";
    // let api_key = "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";

    // let response = client
    //     .post(url)
    //     .header("x-api-key", api_key)
    //     .header("anthropic-version", "2023-06-01")
    //     .header("content-type", "application/json")
    //     .json(&json!({
    //         "model": "claude-3-opus-20240229",
    //         "max_tokens": 1024,
    //         "messages": [
    //             {
    //                 "role": "user",
    //                 "content": "Repeat the following content 5 times"
    //             }
    //         ],
    //         "stream": true
    //     }))
    //     .send()
    //     .await
    //     .expect("to work");

    // if response.status().is_success() {
    //     let body = response.text().await.expect("to work");
    //     println!("Response Body: {}", body);
    // } else {
    //     println!("Request failed with status: {}", response.status());
    // }
}
