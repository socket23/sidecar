//! We want to test if the together ai client is working as expected

use sidecar::llm::{
    clients::{
        togetherai::TogetherAIClient,
        types::{LLMClient, LLMClientCompletionRequest},
    },
    provider::TogetherAIProvider,
};

#[tokio::main]
async fn main() {
    let provider = TogetherAIProvider::new(
        "cc10d6774e67efef2004b85efdb81a3c9ba0b7682cc33d59c30834183502208d".to_owned(),
        "mistralai/Mixtral-8x7B-Instruct-v0.1".to_owned(),
    );

    let client = TogetherAIClient::new(provider);

    let prompt = "<s>[INST] Hi my name is .. what? my name is .. who? my name is... [/INST]";
    let request = LLMClientCompletionRequest::new(
        "mistralai/Mixtral-8x7B-Instruct-v0.1".to_owned(),
        prompt.to_owned(),
        0.5,
        None,
    );
    let response = client.completion(request).await;
    dbg!(&response);
}
