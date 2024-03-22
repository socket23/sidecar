//! Call the endpoints of codestory endpoint

use llm_client::{
    clients::{
        codestory::CodeStoryClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::LLMProviderAPIKeys,
};

#[tokio::main]
async fn main() {
    // let codestory_client =
    //     CodeStoryClient::new("https://codestory-provider-dot-anton-390822.ue.r.appspot.com");
    let codestory_client = CodeStoryClient::new("http://localhost:8080");
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let request = LLMClientCompletionRequest::new(
        LLMType::Gpt4Turbo,
        vec![
            LLMClientMessage::system("you are a python expert".to_owned()),
            LLMClientMessage::user("Can you write 1 to 300 in a new line for me".to_owned()),
        ],
        1.0,
        None,
    )
    .set_max_tokens(2000);
    let response = codestory_client
        .stream_completion(LLMProviderAPIKeys::CodeStory, request, sender)
        .await;
    println!("{:?}", response);
}
