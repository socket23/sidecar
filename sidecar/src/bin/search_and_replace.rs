use llm_client::{
    clients::{
        anthropic::AnthropicClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::AnthropicAPIKey,
};
use sidecar::agentic::symbol::identifier::LLMProperties;

#[tokio::main]
async fn main() {
    let system_prompt = r#"Act as an expert software developer.
Always use best practices when coding.
Respect and use existing conventions, libraries, etc that are already present in the code base.
You are diligent and tireless!
You NEVER leave comments describing code without implementing it!
You always COMPLETELY IMPLEMENT the needed code!
You will be presented with a single file and the code which you can EDIT will be given in a <code_to_edit_section>.
You will be also provided with some extra data, which contains various definitions of symbols which you can use to use the call the correct functions and re-use existing functionality in the code.
Take requests for changes to the supplied code.
If the request is ambiguous, ask questions.

Always reply to the user in the same language they are using.

Once you understand the request you MUST:
1. Decide if you need to propose *SEARCH/REPLACE* edits to any files that haven't been added to the chat. You can create new files without asking. But if you need to propose edits to existing files not already added to the chat, you *MUST* tell the user their full path names and ask them to *add the files to the chat*. End your reply and wait for their approval. You can keep asking if you then decide you need to edit more files.
2. Think step-by-step and explain the needed changes with a numbered list of short sentences put this in a xml tag called <thinking> at the very start of your answer.
3. Describe each change with a *SEARCH/REPLACE block* per the examples below. All changes to files must use this *SEARCH/REPLACE block* format. ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!

All changes to files must use the *SEARCH/REPLACE block* format.

# *SEARCH/REPLACE block* Rules:

Every *SEARCH/REPLACE block* must use this format:
1. The file path alone on a line, verbatim. No bold asterisks, no quotes around it, no escaping of characters, etc.
2. The opening fence and code language, eg: ```rust
3. The start of search block: <<<<<<< SEARCH
4. A contiguous chunk of lines to search for in the existing source code
5. The dividing line: =======
6. The lines to replace into the source code
7. The end of the replace block: >>>>>>> REPLACE
8. The closing fence: ```

Every *SEARCH* section must *EXACTLY MATCH* the existing source code, character for character, including all comments, docstrings, etc.


*SEARCH/REPLACE* blocks will replace *all* matching occurrences.
Include enough lines to make the SEARCH blocks uniquely match the lines to change.

Keep *SEARCH/REPLACE* blocks concise.
Break large *SEARCH/REPLACE* blocks into a series of smaller blocks that each change a small portion of the file.
Include just the changing lines, and a few surrounding lines if needed for uniqueness.
Do not include long runs of unchanging lines in *SEARCH/REPLACE* blocks.

Only create *SEARCH/REPLACE* blocks for files that the user has added to the chat!

To move code within a file, use 2 *SEARCH/REPLACE* blocks: 1 to delete it from its current location, 1 to insert it in the new location.

If you want to put code in a new file, use a *SEARCH/REPLACE block* with:
- A new file path, including dir name if needed
- An empty `SEARCH` section
- The new file's contents in the `REPLACE` section

You are diligent and tireless!
You NEVER leave comments describing code without implementing it!
You always COMPLETELY IMPLEMENT the needed code!
ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!
You always put your thinking in <thinking> section before you suggest *SEARCH/REPLACE* blocks"#;
    let example_messages = vec![
        LLMClientMessage::user(r#"Change get_factorial() to use math.factorial"#.to_owned()),
        LLMClientMessage::assistant(
            r#"<thinking>
To make this change we need to modify `mathweb/flask/app.py` to:

1. Import the math package.
2. Remove the existing factorial() function.
3. Update get_factorial() to call math.factorial instead.
</thinking>

Here are the *SEARCH/REPLACE* blocks:

mathweb/flask/app.py
```python
<<<<<<< SEARCH
from flask import Flask
=======
import math
from flask import Flask
>>>>>>> REPLACE
```

mathweb/flask/app.py
```python
<<<<<<< SEARCH
def factorial(n):
    "compute factorial"

    if n == 0:
        return 1
    else:
        return n * factorial(n-1)

=======
>>>>>>> REPLACE
```

mathweb/flask/app.py
```python
<<<<<<< SEARCH
    return str(factorial(n))
=======
    return str(math.factorial(n))
>>>>>>> REPLACE
```"#
                .to_owned(),
        ),
    ];

    let fs_path = "/Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/tool_box.rs";
    let _file_contents =
        String::from_utf8(tokio::fs::read(&fs_path).await.expect("to work")).expect("to work");

    let extra_data = r#"<extra_data>
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/identifier.rs
#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Deserialize, serde::Serialize)]
pub struct SymbolIdentifier {
    symbol_name: String,
    fs_file_path: Option<String>,
}
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/identifier.rs
impl SymbolIdentifier {
    pub fn new_symbol(symbol_name: &str) -> Self {
    pub fn fs_file_path(&self) -> Option<String> {
    pub fn symbol_name(&self) -> &str {
    pub fn with_file_path(symbol_name: &str, fs_file_path: &str) -> Self {
}
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/ui_event.rs
#[derive(Debug, serde::Serialize)]
pub struct SymbolEventDocumentRequest {
    fs_file_path: String,
    range: Range,
    documentation: String,
}
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/ui_event.rs
impl SymbolEventDocumentRequest {
    pub fn new(fs_file_path: String, range: Range, documentation: String) -> Self {
}
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/agentic/symbol/ui_event.rs
#[derive(Debug, serde::Serialize)]
pub enum SymbolEventSubStep {
    Probe(SymbolEventProbeRequest),
    GoToDefinition(SymbolEventGoToDefinitionRequest),
    Edit(SymbolEventEditRequest),
    Document(SymbolEventDocumentRequest),
}
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/chunking/text_document.rs
#[serde(rename_all = "camelCase")]
pub struct Range {
    start_position: Position,
    end_position: Position,
}
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/chunking/text_document.rs
impl Range {
    // Here we are checking with line and column number values
    /// From byte range helps us get the position while also fixing the
    /// line and the column values which is the position for the byte
    /// This only checks the line without the column for now
    pub fn new(start_position: Position, end_position: Position) -> Self {
    pub fn set_start_byte(&mut self, byte: usize) {
    pub fn set_end_byte(&mut self, byte: usize) {
    pub fn start_position(&self) -> Position {
    pub fn end_position(&self) -> Position {
    pub fn start_byte(&self) -> usize {
    pub fn end_byte(&self) -> usize {
    pub fn start_line(&self) -> usize {
    pub fn end_line(&self) -> usize {
    pub fn start_column(&self) -> usize {
    pub fn end_column(&self) -> usize {
    pub fn get_start_position(&self) -> &Position {
    pub fn get_end_position(&self) -> &Position {
    pub fn set_end_position(&mut self, position: Position) {
    pub fn set_start_position(&mut self, position: Position) {
    pub fn intersection_size(&self, other: &Range) -> usize {
    pub fn contains_line(&self, line: usize) -> bool {
    pub fn len(&self) -> usize {
    pub fn to_tree_sitter_range(&self) -> tree_sitter::Range {
    pub fn for_tree_node(node: &tree_sitter::Node) -> Self {
    pub fn is_contained(&self, other: &Self) -> bool {
    pub fn guard_large_expansion(
        selection_range: Self,
        expanded_range: Self,
        _size: usize,
    ) -> Self {
    pub fn contains_position(&self, position: &Position) -> bool {
    pub fn contains(&self, other: &Range) -> bool {
    pub fn contains_check_line(&self, other: &Range) -> bool {
    pub fn contains_check_line_column(&self, other: &Range) -> bool {
    pub fn from_byte_range(range: std::ops::Range<usize>, line_end_indices: &[u32]) -> Range {
    pub fn byte_size(&self) -> usize {
    pub fn intersects_without_byte(&self, other: &Range) -> bool {
    pub fn minimal_line_distance(&self, other: &Range) -> i64 {
    pub fn check_equality_without_byte(&self, other: &Range) -> bool {
    pub fn line_size(&self) -> i64 {
    pub fn reshape_for_selection(self, edited_code: &str) -> Self {
}
</extra_data>"#.to_owned();
    let user_message = format!(
        r#"
{extra_data}
I have the following code above:
<code_above>
                ),
            ),
        }}
    }}

    pub fn edited_code(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        edited_code: String,
    ) -> Self {{
        Self {{
            request_id,
            event: UIEvent::SymbolEventSubStep(SymbolEventSubStepRequest::edited_code(
                symbol_identifier,
                range,
                fs_file_path,
                edited_code,
            )),
        }}
    }}

    pub fn code_correctness_action(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        tool_use_thinking: String,
    ) -> Self {{
        Self {{
            request_id,
            event: UIEvent::SymbolEventSubStep(SymbolEventSubStepRequest::code_correctness_action(
                symbol_identifier,
                range,
                fs_file_path,
                tool_use_thinking,
            )),
        }}
    }}

    /// Sends the initial search event to the editor
    pub fn initial_search_symbol_event(
        request_id: String,
        symbols: Vec<InitialSearchSymbolInformation>,
    ) -> Self {{
        Self {{
            request_id: request_id.to_owned(),
            event: UIEvent::FrameworkEvent(FrameworkEvent::InitialSearchSymbols(
                InitialSearchSymbolEvent::new(request_id, symbols),
            )),
        }}
    }}
}}

#[derive(Debug, serde::Serialize)]
pub enum UIEvent {{
    SymbolEvent(SymbolEventRequest),
    ToolEvent(ToolInput),
    CodebaseEvent(SymbolInputEvent),
    SymbolLoctationUpdate(SymbolLocation),
    SymbolEventSubStep(SymbolEventSubStepRequest),
    RequestEvent(RequestEvents),
    EditRequestFinished(String),
    FrameworkEvent(FrameworkEvent),
}}

impl From<SymbolEventRequest> for UIEvent {{
    fn from(req: SymbolEventRequest) -> Self {{
        UIEvent::SymbolEvent(req)
    }}
}}

impl From<ToolInput> for UIEvent {{
    fn from(input: ToolInput) -> Self {{
        UIEvent::ToolEvent(input)
    }}
}}

#[derive(Debug, serde::Serialize)]
pub enum SymbolEventProbeRequest {{
    SubSymbolSelection,
    ProbeDeeperSymbol,
    /// The final answer for the probe is sent via this event
    ProbeAnswer(String),
}}

#[derive(Debug, serde::Serialize)]
pub struct SymbolEventGoToDefinitionRequest {{
    fs_file_path: String,
    range: Range,
    thinking: String,
}}

impl SymbolEventGoToDefinitionRequest {{
    fn new(fs_file_path: String, range: Range, thinking: String) -> Self {{
        Self {{
            fs_file_path,
            range,
            thinking,
        }}
    }}
}}

#[derive(Debug, serde::Serialize)]
pub struct RangeSelectionForEditRequest {{
    range: Range,
    fs_file_path: String,
}}

impl RangeSelectionForEditRequest {{
    pub fn new(range: Range, fs_file_path: String) -> Self {{
        Self {{
            range,
            fs_file_path,
        }}
    }}
}}

#[derive(Debug, serde::Serialize)]
pub struct InsertCodeForEditRequest {{
    range: Range,
    fs_file_path: String,
}}

#[derive(Debug, serde::Serialize)]
pub struct EditedCodeForEditRequest {{
    range: Range,
    fs_file_path: String,
    new_code: String,
}}

impl EditedCodeForEditRequest {{
    pub fn new(range: Range, fs_file_path: String, new_code: String) -> Self {{
        Self {{
            range,
            fs_file_path,
            new_code,
        }}
    }}
}}

#[derive(Debug, serde::Serialize)]
pub struct CodeCorrectionToolSelection {{
    range: Range,
    fs_file_path: String,
    tool_use_thinking: String,
}}

impl CodeCorrectionToolSelection {{
    pub fn new(range: Range, fs_file_path: String, tool_use_thinking: String) -> Self {{
        Self {{
            range,
            fs_file_path,
            tool_use_thinking,
        }}
    }}
}}

/// We have range selection and then the edited code, we should also show the
/// events which the AI is using for the tool correction and whats it is planning
/// on doing for that
#[derive(Debug, serde::Serialize)]
pub enum SymbolEventEditRequest {{
    RangeSelectionForEdit(RangeSelectionForEditRequest),
    /// We might be inserting code at a line which is a new symbol by itself
    InsertCode(InsertCodeForEditRequest),
    EditCode(EditedCodeForEditRequest),
    CodeCorrectionTool(CodeCorrectionToolSelection),
}}
#[derive(Debug, serde::Serialize)]
pub struct SymbolEventDocumentRequest {{
    fs_file_path: String,
    range: Range,
    documentation: String,
}}

impl SymbolEventDocumentRequest {{
    pub fn new(fs_file_path: String, range: Range, documentation: String) -> Self {{
        Self {{
            fs_file_path,
            range,
            documentation,
        }}
    }}
}}

#[derive(Debug, serde::Serialize)]
pub enum SymbolEventSubStep {{
    Probe(SymbolEventProbeRequest),
    GoToDefinition(SymbolEventGoToDefinitionRequest),
    Edit(SymbolEventEditRequest),
    Document(SymbolEventDocumentRequest),
}}

#[derive(Debug, serde::Serialize)]
pub struct SymbolEventSubStepRequest {{
    symbol_identifier: SymbolIdentifier,
    event: SymbolEventSubStep,
}}
</code_above>
I have the following code below:
<code_below>

#[derive(Debug, serde::Serialize)]
pub struct RequestEventProbeFinished {{
    reply: String,
}}

impl RequestEventProbeFinished {{
    pub fn new(reply: String) -> Self {{
        Self {{ reply }}
    }}
}}

#[derive(Debug, serde::Serialize)]
pub enum RequestEvents {{
    ProbingStart,
    ProbeFinished(RequestEventProbeFinished),
}}

#[derive(Debug, serde::Serialize)]
pub struct InitialSearchSymbolInformation {{
    symbol_name: String,
    fs_file_path: Option<String>,
    is_new: bool,
    thinking: String,
    // send over the range of this symbol
    range: Option<Range>,
}}

impl InitialSearchSymbolInformation {{
    pub fn new(
        symbol_name: String,
        fs_file_path: Option<String>,
        is_new: bool,
        thinking: String,
        range: Option<Range>,
    ) -> Self {{
        Self {{
            symbol_name,
            fs_file_path,
            is_new,
            thinking,
            range,
        }}
    }}
}}

#[derive(Debug, serde::Serialize)]
pub struct InitialSearchSymbolEvent {{
    request_id: String,
    symbols: Vec<InitialSearchSymbolInformation>,
}}

impl InitialSearchSymbolEvent {{
    pub fn new(request_id: String, symbols: Vec<InitialSearchSymbolInformation>) -> Self {{
        Self {{
            request_id,
            symbols,
        }}
    }}
}}

#[derive(Debug, serde::Serialize)]
pub enum FrameworkEvent {{
    RepoMapGenerationStart(String),
    RepoMapGenerationFinished(String),
    LongContextSearchStart(String),
</code_below>
I have the following code in selection to edit:
<code_to_edit_section>
FILEPATH: {fs_path}
impl SymbolEventSubStepRequest {{
    pub fn new(symbol_identifier: SymbolIdentifier, event: SymbolEventSubStep) -> Self {{
        Self {{
            symbol_identifier,
            event,
        }}
    }}

    pub fn probe_answer(symbol_identifier: SymbolIdentifier, answer: String) -> Self {{
        Self {{
            symbol_identifier,
            event: SymbolEventSubStep::Probe(SymbolEventProbeRequest::ProbeAnswer(answer)),
        }}
    }}

    pub fn go_to_definition_request(
        symbol_identifier: SymbolIdentifier,
        fs_file_path: String,
        range: Range,
        thinking: String,
    ) -> Self {{
        Self {{
            symbol_identifier,
            event: SymbolEventSubStep::GoToDefinition(SymbolEventGoToDefinitionRequest::new(
                fs_file_path,
                range,
                thinking,
            )),
        }}
    }}

    pub fn range_selection_for_edit(
        symbol_identifier: SymbolIdentifier,
        fs_file_path: String,
        range: Range,
    ) -> Self {{
        Self {{
            symbol_identifier,
            event: SymbolEventSubStep::Edit(SymbolEventEditRequest::RangeSelectionForEdit(
                RangeSelectionForEditRequest::new(range, fs_file_path),
            )),
        }}
    }}

    pub fn edited_code(
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        edited_code: String,
    ) -> Self {{
        Self {{
            symbol_identifier,
            event: SymbolEventSubStep::Edit(SymbolEventEditRequest::EditCode(
                EditedCodeForEditRequest::new(range, fs_file_path, edited_code),
            )),
        }}
    }}

    pub fn code_correctness_action(
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        tool_use_thinking: String,
    ) -> Self {{
        Self {{
            symbol_identifier,
            event: SymbolEventSubStep::Edit(SymbolEventEditRequest::CodeCorrectionTool(
                CodeCorrectionToolSelection::new(range, fs_file_path, tool_use_thinking),
            )),
        }}
    }}
}}
<code_to_edit_section>

<user_instruction>
Add a new method to SymbolEventSubStepRequest for creating a Document request
</user_instruction>

FILEPATH: {fs_path}"#
    );

    let llm_request = LLMClientCompletionRequest::new(
        LLMType::ClaudeSonnet,
        vec![LLMClientMessage::system(system_prompt.to_owned())]
            .into_iter()
            .chain(example_messages)
            .chain(vec![LLMClientMessage::user(user_message.to_owned())])
            .collect::<Vec<_>>(),
        0.2,
        None,
    );

    let llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        llm_client::provider::LLMProvider::Anthropic,
        llm_client::provider::LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned())),
    );

    let client = AnthropicClient::new();
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let start_time = std::time::Instant::now();
    let response = client
        .stream_completion(llm_properties.api_key().clone(), llm_request, sender)
        .await;
    println!("time_taken:{}", start_time.elapsed().as_secs());
    println!("{}", response.expect("to work"));
}
