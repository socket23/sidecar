use async_openai::{
    config::AzureConfig,
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs},
    Client,
};
use futures::StreamExt;
use llm_client::{
    clients::togetherai::TogetherAIClient, provider::AzureConfig as ProviderAzureConfig,
};
use llm_client::{
    clients::{
        openai::OpenAIClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage},
    },
    provider::TogetherAIProvider,
};

#[tokio::main]
async fn main() {
    let togetherai = TogetherAIClient::new();
    let api_key = llm_client::provider::LLMProviderAPIKeys::TogetherAI(TogetherAIProvider {
        api_key: "cc10d6774e67efef2004b85efdb81a3c9ba0b7682cc33d59c30834183502208d".to_owned(),
    });
    let request = LLMClientCompletionRequest::new(
        llm_client::clients::types::LLMType::Mixtral,
        vec![LLMClientMessage::system("message".to_owned())],
        1.0,
        None,
    );
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = togetherai.stream_completion(api_key, request, sender).await;
    dbg!(&response);
}
