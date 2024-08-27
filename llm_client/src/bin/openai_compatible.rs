use llm_client::clients::{
    openai_compatible::OpenAICompatibleClient,
    types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage},
};
use llm_client::provider::OpenAICompatibleConfig;

#[tokio::main]
async fn main() {
    let api_key = std::env::var("api_key").expect("to be present");
    let api_base = std::env::var("api_base").expect("to work");
    let openai_client = OpenAICompatibleClient::new();
    let api_key =
        llm_client::provider::LLMProviderAPIKeys::OpenAICompatible(OpenAICompatibleConfig {
            api_key,
            api_base,
        });
    let request = LLMClientCompletionRequest::new(
        llm_client::clients::types::LLMType::Custom("gpt-4o".to_owned()),
        vec![LLMClientMessage::system(
            "tell me how to add 2 numbers in rust".to_owned(),
        )],
        1.0,
        None,
    );
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = openai_client
        .stream_completion(api_key, request, sender)
        .await;
    dbg!(&response);
}
