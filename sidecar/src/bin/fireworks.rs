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
    let system_message = r#"You are an expert senior software engineer whose is going to check if we should proceed with making changes ONLY to ProbeStopRequest given the reason for edit picked by another junior engineer.

- You are working with a junior engineer who is a fast coder but might repeat work they have already done.
- The edit is part of a bigger plan to accomplish the goal of the user which is provided in <user_instruction>
- We are right now focussed on ProbeStopRequest and can only make changes to ProbeStopRequest
- Your job is to look at the code present in <code_which_we_can_edit> section and the reason for editing which is given in <reason_to_edit> section and reply with true or false in xml format (which we will show you) and your thinking
- Before replying you should think for a bit less than 2 sentences and then decide if you want to edit or not and put `true` or `false` in the <should_edit> section
- You have to be extremely careful when deciding if we can proceed with the edit and take the following points into consideration:
- - We are right now working with ProbeStopRequest so if the edit instruction does not require changes to ProbeStopRequest we SHOULD REJECT it
- - If the <reason_to_edit> is talking about creating or editing part of the code which DOES NOT belong to ProbeStopRequest we SHOULD NOT go forward with the edit
- - If the changes to ProbeStopRequest are already done to satisfy the task then we should reject it
- - If the instruction is to introduce a new structure or functionality which does not belong to ProbeStopRequest you should NOT ALLOW this edit to happen
- - Just be careful since you are the senior engineer and you have to provide feedback to the junior engineer and let them know the reason for your verdict which will be true or false
- - Think step by step first and put your thinking in <thinking> section

Now to show you the reply format:
<reply>
<thinking>
{your thoughts here if the edit reason is correct and we can proceed with the editing ProbeStopRequest}
</thinking>
<should_edit>
{true or false}
</should_edit>
</reply>

The input will be in the following format:
<user_instruction>
{the goal of the user}
</user_instruction>
<reason_to_edit>
{the reason for selecting this section of code for editing}
</reason_to_edit>
<code_symbol_we_can_edit>
{the code symbol which we can edit right now, anything beyond this can not be edited}
</code_symbol_we_can_edit>
<code_which_can_be_edited>
{code which we want to edit}
</code_which_can_be_edited>

We are also going to show you an example:
<user_instruction>
I want to support other kind of mathematical operations
</user_instruction>
<reason_to_edit>
we should look at how we accept input for add to figure out how to implement subtract
</reason_to_edit>
<code_symbol_we_can_edit>
add
</code_symbol_we_can_edit>
<code_which_can_be_edited>
```py
FILEPATH: maths.py
def add(a: int, b: int) -> int:
    return a + b
```
</code_which_can_be_edited>

Your reply should be:
<reply>
<thinking>
the reason to edit talks about implementing subtract but we can only edit add, so we should not edit shit section of the code
</thinking>
<should_edit>
false
</should_edit>
</reply>

Another example:
<user_instruction>
support postgres db similar to how we use sqlite
</user_instruction>
<reason_to_edit>
we need to define a new request object for postgres similar to how `SqliteRequest` is created. This will be used to handle postgres db
</reason_to_edit>
<code_symbol_we_can_edit>
SqliteRequest
</code_symbol_we_can_edit>
<code_which_can_be_edited>
```py
FILEPATH: sqlite.py
class SqliteRequest:
    id
    query
```
</code_which_can_be_edited>

Your reply should be:
<reply>
<thinking>
the reason to edit is to define a new request object but since we can only edit `SqliteRequest` so this edit is NOT relevant and is more of a request to understand how SqliteRequest works
</thinking>
<should_edit>
false
</should_edit>
</reply>"#;
    let user_message = r#"<user_instruction>
Add support for a new stop_code_editing endpoint and implement it similar to probing stop
</user_instruction>
<reason_to_edit>
We need to define a new request structure for the `stop_code_editing` endpoint similar to `ProbeStopRequest`. This will be used to handle the new endpoint.
</reason_to_edit>
<code_symbol_we_can_edit>
ProbeStopRequest
</code_symbol_we_can_edit>
<code_which_can_be_edited>
```rust
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/webserver/agentic.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeStopRequest {
    request_id: String,
}
```
</code_which_can_be_edited>"#;
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
        0.0,
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
