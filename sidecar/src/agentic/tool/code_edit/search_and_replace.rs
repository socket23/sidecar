//! Contains the struct for search and replace style editing

use async_trait::async_trait;
use std::sync::Arc;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::{
    agentic::{
        symbol::identifier::LLMProperties,
        tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
    },
    chunking::text_document::Range,
};

const SURROUNDING_CONTEXT_LIMIT: usize = 200;

#[derive(Debug)]
pub struct SearchAndReplaceEditingResponse {
    response: String,
}

impl SearchAndReplaceEditingResponse {
    pub fn new(response: String) -> Self {
        Self { response }
    }

    pub fn response(&self) -> &str {
        &self.response
    }
}

#[derive(Debug, Clone)]
pub struct SearchAndReplaceEditingRequest {
    fs_file_path: String,
    // TODO(skcd): we use this to detect the range where we want to perform the edits
    _edit_range: Range,
    context_in_edit_selection: String,
    code_above: Option<String>,
    code_below: Option<String>,
    extra_data: String,
    llm_properties: LLMProperties,
    language: String,
    new_symbols: Option<String>,
    instructions: String,
    root_request_id: String,
}

impl SearchAndReplaceEditingRequest {
    pub fn new(
        fs_file_path: String,
        edit_range: Range,
        context_in_edit_selection: String,
        code_above: Option<String>,
        code_below: Option<String>,
        extra_data: String,
        llm_properties: LLMProperties,
        language: String,
        new_symbols: Option<String>,
        instructions: String,
        root_request_id: String,
    ) -> Self {
        Self {
            fs_file_path,
            _edit_range: edit_range,
            context_in_edit_selection,
            code_above,
            code_below,
            extra_data,
            llm_properties,
            language,
            new_symbols,
            instructions,
            root_request_id,
        }
    }
}

pub struct SearchAndReplaceEditing {
    llm_client: Arc<LLMBroker>,
    _fail_over_llm: LLMProperties,
}

impl SearchAndReplaceEditing {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            _fail_over_llm: fail_over_llm,
        }
    }

    fn system_message(&self, language: &str) -> String {
        format!(r#"Act as an expert software developer.
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
2. The opening fence and code language, eg: ```{language}
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
You always put your thinking in <thinking> section before you suggest *SEARCH/REPLACE* blocks"#).to_owned()
    }

    fn extra_data(&self, extra_data: &str) -> String {
        format!(
            r#"This is the extra data which you can use:
<extra_data>
{extra_data}
</extra_data>"#
        )
    }

    fn above_selection(&self, above_selection: Option<&str>) -> Option<String> {
        if let Some(above_selection) = above_selection {
            Some(format!(
                r#"<code_above>
{above_selection}
</code_above>"#
            ))
        } else {
            None
        }
    }

    fn below_selection(&self, below_selection: Option<&str>) -> Option<String> {
        if let Some(below_selection) = below_selection {
            Some(format!(
                r#"<code_below>
{below_selection}
</code_below>"#
            ))
        } else {
            None
        }
    }

    fn selection_to_edit(&self, selection_to_edit: &str) -> String {
        format!(
            r#"<code_to_edit_selection>
{selection_to_edit}
</code_to_edit_selection>"#
        )
    }

    fn user_message(&self, context: SearchAndReplaceEditingRequest) -> String {
        let extra_data = self.extra_data(&context.extra_data);
        let above = self.above_selection(
            context
                .code_above
                .map(|code_above| {
                    // limit it to 100 lines from the start
                    let mut lines = code_above.lines().collect::<Vec<_>>();
                    lines.reverse();
                    lines.truncate(SURROUNDING_CONTEXT_LIMIT);
                    lines.reverse();
                    lines.join("\n")
                })
                .as_deref(),
        );
        let below = self.below_selection(
            context
                .code_below
                .map(|code_below| {
                    let mut lines = code_below.lines().collect::<Vec<_>>();
                    lines.truncate(SURROUNDING_CONTEXT_LIMIT / 3);
                    lines.join("\n")
                })
                .as_deref(),
        );
        let in_range = self.selection_to_edit(&context.context_in_edit_selection);
        let mut user_message = "".to_owned();
        if let Some(extra_symbols) = context.new_symbols.clone() {
            user_message = user_message
                + &format!(
                    r#"<extra_symbols_will_be_created>
{extra_symbols}
</extra_symbols_will_be_created>"#
                );
        }
        user_message = user_message + &extra_data + "\n";
        if let Some(above) = above {
            user_message = user_message + &above + "\n";
        }
        if let Some(below) = below {
            user_message = user_message + &below + "\n";
        }
        user_message = user_message + &in_range + "\n";
        let instructions = context.instructions;
        let fs_file_path = context.fs_file_path;
        user_message = user_message
            + &format!(
                r#"Only edit the code in <code_to_edit_selection> my instructions are:
<user_instruction>
{instructions}
</user_insturction>

<fs_file_path>
{fs_file_path}
</fs_file_path>"#
            );
        user_message
    }

    fn example_messages(&self) -> Vec<LLMClientMessage> {
        vec![
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
        ]
    }
}

#[async_trait]
impl Tool for SearchAndReplaceEditing {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.should_search_and_replace_editing()?;
        let llm_properties = context.llm_properties.clone();
        let root_request_id = context.root_request_id.to_owned();
        let system_message = LLMClientMessage::system(self.system_message(&context.language));
        let user_message = LLMClientMessage::user(self.user_message(context));
        let example_messages = self.example_messages();
        let request = LLMClientCompletionRequest::new(
            llm_properties.llm().clone(),
            vec![system_message]
                .into_iter()
                .chain(example_messages)
                .chain(vec![user_message])
                .collect(),
            0.2,
            None,
        );
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let response = self
            .llm_client
            .stream_completion(
                llm_properties.api_key().clone(),
                request,
                llm_properties.provider().clone(),
                vec![
                    (
                        "event_type".to_owned(),
                        "search_and_replace_editing".to_owned(),
                    ),
                    ("root_id".to_owned(), root_request_id),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await
            .map_err(|e| ToolError::LLMClientError(e))?;
        Ok(ToolOutput::search_and_replace_editing(
            SearchAndReplaceEditingResponse::new(response),
        ))
    }
}
