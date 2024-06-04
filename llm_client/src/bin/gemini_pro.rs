use llm_client::{
    clients::{
        gemini_pro::GeminiProClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::{GeminiProAPIKey, LLMProviderAPIKeys},
};

#[tokio::main]
async fn main() {
    // let gemini_pro_client = GeminiProClient::new();
    // let api_key = LLMProviderAPIKeys::GeminiPro(GeminiProAPIKey::new(
    //     "ya29.a0AXooCguiRZP_3G8vUxvkKgrEfcTyGu-xdqdv5SyXsgvWKuaxJSjjTTRH7_cvzsYrOqyyZ_P7-gQFw_L1VRsl1xITfFsvTbVJLsaYUqVGBwKNG4d8obg6OQctm36QxeWwTGYNvke10k_oMW1ygkhIzjIsogk_d_PnBfecn8TubmkaCgYKAeMSARESFQHGX2MiUhp9vFKvNq1Lp7CMO-x2pA0178".to_owned(),
    //     "anton-390822".to_owned(),
    // ));
    // let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    // let request = LLMClientCompletionRequest::from_messages(
    //     vec![
    //         LLMClientMessage::system("You are an expert software engineer".to_owned()),
    //         LLMClientMessage::user(
    //             "Help me write a function in rust which adds 2 numbers".to_owned(),
    //         ),
    //     ],
    //     LLMType::GeminiProFlash,
    // );
    // let response = gemini_pro_client
    //     .stream_completion(api_key, request, sender)
    //     .await;
    // println!("{:?}", response);
    let context = std::fs::read_to_string("/tmp/repo_map").expect("to work");
    let gemini_pro_clint = GeminiProClient::new();
    let result = gemini_pro_clint
        .count_tokens(&context, "anton-390822", "ya29.a0AXooCgsNY8zDCrOkND_hjC3kz0KxH-Czqem7XwlAJQf4n3ymjseP8x9K3yKDdrLhOHgSyw7EMKoaDW7CQHeRT-XZJU8ifPyEvdrb7VLvuBegTEoJ5k-k2qci6G6OtSf7ae32364bwo3Xghfdg6DrHiTeFHc1WQES_LNCibuVBXYaCgYKAV4SARESFQHGX2MioTj4haqaMXaEdRWn8TvIDg0178", "gemini-1.5-flash-001")
        .await;
    println!("{:?}", &result);
}
