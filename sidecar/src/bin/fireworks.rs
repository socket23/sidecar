use llm_client::{
    clients::{
        fireworks::FireworksAIClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::{FireworksAPIKey, LLMProvider, LLMProviderAPIKeys},
};
use sidecar::agentic::symbol::identifier::LLMProperties;

#[tokio::main]
async fn main() {
    let system_message = r#"You are an expert senior software engineer whose is going to check if the task the junior engineer is asking needs to be done.

- You are working with a junior engineer who is a fast coder but might repeat work they have already done.
- Your job is to look at the code present in <code_to_edit> section and the task which is given in <task> section and reply with yes or no in xml format (which we will show you) and your thinking
- This is part of a greater change which a user wants to get done on the codebase which is given in <user_instruction>
- Before replying you should think for a bit less than 5 sentences and then decide if you want to edit or not and put `true` or `false` in the should_edit section
- You should be careful to decide the following:
- - We are right now working with ProbeStopRequest so if the change instruction is not related to it we should reject it
- - If the changes to ProbeStopRequest are already done to satisfy the task then we should reject it
- - If the changes are absolutely necessary then we should do it
- - Just be careful since you are the senior engineer and you have to provide feedback to the junior engineer and let them know the reason for your verdict on true or false

Now to show you the reply format:
<reply>
<thinking>
{your thoughts here if the edit should be done}
</thinking>
<should_edit>
{true or false}
</should_edit>
</reply>

The input will be in the following format:
<user_instruction>
</user_instruction>
<task>
</task>
<code_to_edit>
</code_to_edit>"#;
    let user_message = r#"<user_instruction>
Add support for a new stop_code_editing endpoint and implement it similar to probing stop
</user_instruction>
<task>
We need to add a new endpoint for stopping code editing and implement it similar to the existing probe_request_stop endpoint.
</task>
<code_to_edit>
```rust
FILEPATH: /Users/skcd/scratch/sidecar/sidecar/src/webserver/agentic.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeStopRequest {
    request_id: String,
}
```
</code_to_edit>"#;
    // let gemini_llm_prperties = LLMProperties::new(
    //     LLMType::GeminiPro,
    //     LLMProvider::GoogleAIStudio,
    //     LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
    //         "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
    //     )),
    // );
    let fireworks_ai = LLMProperties::new(
        LLMType::Llama3_1_8bInstruct,
        LLMProvider::FireworksAI,
        LLMProviderAPIKeys::FireworksAI(FireworksAPIKey::new(
            "s8Y7yIXdL0lMeHHgvbZXS77oGtBAHAsfsLviL2AKnzuGpg1n".to_owned(),
        )),
    );
    let llm_request = LLMClientCompletionRequest::new(
        fireworks_ai.llm().clone(),
        vec![
            LLMClientMessage::system(system_message.to_owned()),
            LLMClientMessage::user(user_message.to_owned()),
        ],
        0.2,
        None,
    );
    // let client = GoogleAIStdioClient::new();
    let client = FireworksAIClient::new();
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let start_instant = std::time::Instant::now();
    let response = client
        .stream_completion(fireworks_ai.api_key().clone(), llm_request, sender)
        .await;
    println!(
        "response {}:\n{}",
        start_instant.elapsed().as_millis(),
        response.expect("to work always")
    );
}
