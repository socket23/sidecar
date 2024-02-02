use llm_client::{
    clients::{
        togetherai::TogetherAIClient,
        types::{LLMClient, LLMClientCompletionStringRequest, LLMType},
    },
    provider::{LLMProviderAPIKeys, TogetherAIProvider},
};

#[tokio::main]
async fn main() {
    let api_key = LLMProviderAPIKeys::TogetherAI(TogetherAIProvider {
        api_key: "cc10d6774e67efef2004b85efdb81a3c9ba0b7682cc33d59c30834183502208d".to_owned(),
    });
    let togetherai = TogetherAIClient::new();
    let prompt =
        "<PRE> # non recursive\ndef compute_gcd(x, y): <SUF>return result <MID>".to_owned();
    let request =
        LLMClientCompletionStringRequest::new(LLMType::CodeLlama13BInstruct, prompt, 0.2, None);
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = togetherai
        .stream_prompt_completion(api_key, request, sender)
        .await;
    println!("{}", response.expect("to work"));
}
