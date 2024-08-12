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
    let system_message = r#"You are a powerful code filtering engine. You must order the code snippets in the order in you want to edit them, and only those code snippets which should be edited.
- The code snippets will be provided to you in <code_snippet> section which will also have an id in the <id> section.
- You should select a code section for editing if and only if you want to make changes to that section.
- You are also given a list of extra symbols in <extra_symbols> which will be provided to you while making the changes, use this to be more sure about your reason for selection.
- Adding new functionality is a very valid reason to select a sub-section for editing.
- Editing or deleting some code is also a very valid reason for selecting a code section for editing.
- First think step by step on how you want to go about selecting the code snippets which are relevant to the user query in max 50 words.
- If you want to edit the code section with id 0 then you must output in the following format:
<code_to_edit_list>
<code_to_edit>
<id>
0
</id>
<reason_to_edit>
{your reason for editing}
</reason_to_edit>
</code_to_edit>
</code_to_edit_list>

- If you want to edit more code sections follow the similar pattern as described above and as an example again:
<code_to_edit_list>
<code_to_edit>
<id>
{id of the code snippet you are interested in}
</id>
<reason_to_edit>
{your reason for editing}
</reason_to_edit>
</code_to_edit>
{... more code sections here which you might want to select}
</code_to_edit_list>

- The <id> section should ONLY contain an id from the listed code snippets.


Here is an example contained in the <example> section.

<example>
<user_query>
We want to add a new method to add a new shipment made by the company.
</user_query>

<rerank_list>
<rerank_entry>
<id>
0
</id>
<content>
Code Location: company.rs
```rust
struct Company {
    name: String,
    shipments: usize,
    size: usize,
}
```
</content>
</rerank_entry>
<rerank_entry>
<id>
1
</id>
<content>
Code Location: company_metadata.rs
```rust
impl Compnay {
    fn name(&self) -> &str {
        &self.name
    }

    fn size(&self) -> usize {
        self.size
    }
}
</content>
</rerank_entry>
<rerank_entry>
<id>
2
</id>
<content>
Code Location: company_shipments.rs
```rust
impl Company {
    fn get_snipments(&self) -> usize {
        self.shipments
    }
}
```
</content>
</rerank_entry>
</rerank_list>

Your reply should be:

<thinking>
The company_shipment implementation block handles everything related to the shipments of the company, so we want to edit that.
</thinking>

<code_to_edit_list>
<code_to_edit>
<id>
2
</id>
<reason_to_edit>
The company_shipment.rs implementation block of Company contains all the relevant code for the shipment tracking of the Company, so that's what we want to edit.
</reason_to_edit>
<id>
</code_to_edit>
</code_to_edit_list>
</example>

This example is for reference. You must strictly follow the format show in the example when replying.
Please provide the list of symbols which you want to edit."#;
    let user_message = r#"<user_query>
Add a new method to UIEventWithID for creating a Document event
</user_query>

<extra_symbols>
<symbol>
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/ui_event.rs
SymbolEventDocumentRequest
</symbol>
</extra_symbols>

<rerank_entry>
<id>
0
</id>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/ui_event.rs:12-16
</file_path>
<content>
```
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/ui_event.rs
#[derive(Debug, serde::Serialize)]
pub struct UIEventWithID {
    request_id: String,
    event: UIEvent,
}
```
</content>
</rerank_entry>
<rerank_entry>
<id>
1
</id>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/ui_event.rs:19-190
</file_path>
<content>
```
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/ui_event.rs
impl UIEventWithID {
    /// Repo map search start
    /// Repo map generation end
    /// Sends the initial search event to the editor
    pub fn start_long_context_search(request_id: String) -> Self {
    pub fn finish_long_context_search(request_id: String) -> Self {
    pub fn finish_edit_request(request_id: String) -> Self {
    pub fn from_tool_event(request_id: String, input: ToolInput) -> Self {
    pub fn repo_map_gen_start(request_id: String) -> Self {
    pub fn repo_map_gen_end(request_id: String) -> Self {
    pub fn from_symbol_event(request_id: String, input: SymbolEventRequest) -> Self {
    pub fn for_codebase_event(request_id: String, input: SymbolInputEvent) -> Self {
    pub fn symbol_location(request_id: String, symbol_location: SymbolLocation) -> Self {
    pub fn sub_symbol_step(
        request_id: String,
        sub_symbol_request: SymbolEventSubStepRequest,
    ) -> Self {
    pub fn probe_answer_event(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        probe_answer: String,
    ) -> Self {
    pub fn probing_started_event(request_id: String) -> Self {
    pub fn probing_finished_event(request_id: String, response: String) -> Self {
    pub fn range_selection_for_edit(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
    ) -> Self {
    pub fn edited_code(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        edited_code: String,
    ) -> Self {
    pub fn code_correctness_action(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        tool_use_thinking: String,
    ) -> Self {
    pub fn initial_search_symbol_event(
        request_id: String,
        symbols: Vec<InitialSearchSymbolInformation>,
    ) -> Self {
}
```
</content>
</rerank_entry>"#;
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
    let _few_shot_user_instruction = r#"<code_in_selection>
```py
def add_values(a, b):
    return a + b

def subtract(a, b):
    return a - b
```
</code_in_selection>

<code_changes_outline>
def add_values(a, b, logger):
    logger.info(a, b)
    # rest of the code

def subtract(a, b, logger):
    logger.info(a, b)
    # rest of the code
</code_changes_outline>"#;
    let _few_shot_output = r#"<reply>
```py
def add_values(a, b, logger):
    logger.info(a, b)
    return a + b

def subtract(a, b, logger):
    logger.info(a, b)
    return a - b
```
</reply>"#;
    let llm_request = LLMClientCompletionRequest::new(
        fireworks_ai.llm().clone(),
        vec![
            LLMClientMessage::system(system_message.to_owned()),
            // LLMClientMessage::user(few_shot_user_instruction.to_owned()),
            // LLMClientMessage::assistant(few_shot_output.to_owned()),
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
