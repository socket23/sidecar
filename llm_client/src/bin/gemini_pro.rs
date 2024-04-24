use llm_client::{
    clients::{
        gemini_pro::GeminiProClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::{GeminiProAPIKey, LLMProviderAPIKeys},
};

#[tokio::main]
async fn main() {
    let gemini_pro_client = GeminiProClient::new();
    let api_key = LLMProviderAPIKeys::GeminiPro(GeminiProAPIKey::new(
        "ya29.a0Ad52N3-fixk_UEEgKks633q1SaF6YmrY5vt9VroX40j7nqfU5Ny4S_aLH-AzNDXPfAGxOQXGMSD_LTlQYgSYQYEtKlIWIulI1HD9o9wNSkbGP_EawaEhf4UNZe8hwKDQfv9h2727V3on25fLjS_YsRfJTR3Iz9F-ZOQv6tBAfwaCgYKAZMSARESFQHGX2MiNXTyT7SzpY5qnOijIPOc4A0177".to_owned(),
        "anton-390822".to_owned(),
    ));
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let request = LLMClientCompletionRequest::from_messages(
        vec![
            LLMClientMessage::system("You are an expert software engineer".to_owned()),
            LLMClientMessage::user(
                "Help me write a function in rust which adds 2 numbers".to_owned(),
            ),
        ],
        LLMType::GeminiPro,
    );
    let response = gemini_pro_client
        .stream_completion(api_key, request, sender)
        .await;
    println!("{:?}", response);
}
