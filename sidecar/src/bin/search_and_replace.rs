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
Take requests for changes to the supplied code.
If the request is ambiguous, ask questions.

Always reply to the user in the same language they are using.

Once you understand the request you MUST:
1. Decide if you need to propose *SEARCH/REPLACE* edits to any files that haven't been added to the chat. You can create new files without asking. But if you need to propose edits to existing files not already added to the chat, you *MUST* tell the user their full path names and ask them to *add the files to the chat*. End your reply and wait for their approval. You can keep asking if you then decide you need to edit more files.
2. Think step-by-step and explain the needed changes with a numbered list of short sentences.
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
ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!"#;
    let example_messages = vec![
        LLMClientMessage::user(r#"Change get_factorial() to use math.factorial"#.to_owned()),
        LLMClientMessage::assistant(
            r#"To make this change we need to modify `mathweb/flask/app.py` to:

1. Import the math package.
2. Remove the existing factorial() function.
3. Update get_factorial() to call math.factorial instead.

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

    let user_message = format!(
        r#"FILEPATH: {fs_path}
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

    pub fn document_request(
        symbol_identifier: SymbolIdentifier,
        fs_file_path: String,
        range: Range,
        documentation: String,
    ) -> Self {{
        Self {{
            symbol_identifier,
            event: SymbolEventSubStep::Document(SymbolEventDocumentRequest::new(
                fs_file_path,
                range,
                documentation,
            )),
        }}
    }}
}}
Add a new method to SymbolEventSubStepRequest for creating a Document request
Please only make changes just for the instruction I have provided giving me no other examples and following the instruction to the letter.

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
