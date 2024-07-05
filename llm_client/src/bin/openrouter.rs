use llm_client::{
    clients::{
        open_router::OpenRouterClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage},
    },
    provider::OpenRouterAPIKey,
};

#[tokio::main]
pub async fn main() {
    let client = OpenRouterClient::new();
    let api_key = llm_client::provider::LLMProviderAPIKeys::OpenRouter(OpenRouterAPIKey::new(
        "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".to_owned(),
    ));
    let request = LLMClientCompletionRequest::new(
        llm_client::clients::types::LLMType::ClaudeHaiku,
        vec![
            LLMClientMessage::system("you are an expert at saying hi".to_owned()),
            LLMClientMessage::user("what can you tell me about yourself".to_owned()),
        ],
        0.2,
        None,
    );
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = client.stream_completion(api_key, request, sender).await;
    println!("{:?}", response);
}
