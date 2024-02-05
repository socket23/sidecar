use llm_client::clients::types::LLMClient;
use llm_client::{
    clients::{ollama::OllamaClient, types::LLMClientCompletionStringRequest},
    provider::{LLMProviderAPIKeys, OllamaProvider},
};

#[tokio::main]
async fn main() {
    let api_key = LLMProviderAPIKeys::Ollama(OllamaProvider {});
    let client = OllamaClient::new();
    let prompt =
        "<｜fim▁begin｜>// Path: testing.ts\nfunction subtract(a<｜fim▁hole｜>)<｜fim▁end｜>";
    let request = LLMClientCompletionStringRequest::new(
        llm_client::clients::types::LLMType::DeepSeekCoder6BInstruct,
        prompt.to_owned(),
        0.2,
        None,
    );
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = client
        .stream_prompt_completion(api_key, request, sender)
        .await;
    println!("{}", response.expect("to work"));
}
