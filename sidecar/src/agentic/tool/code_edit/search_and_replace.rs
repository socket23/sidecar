//! Contains the struct for search and replace style editing

use async_trait::async_trait;
use futures::{lock::Mutex, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::UnboundedSender, Semaphore};

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::{
    agentic::{
        symbol::{
            identifier::{LLMProperties, SymbolIdentifier},
            ui_event::{EditedCodeStreamingRequest, UIEventWithID},
        },
        tool::{
            errors::ToolError, input::ToolInput, lsp::open_file::OpenFileRequest,
            output::ToolOutput, r#type::Tool,
        },
    },
    chunking::text_document::{Position, Range},
};

const _SURROUNDING_CONTEXT_LIMIT: usize = 200;

struct DropDetector<T>(T);

impl<T> Drop for DropDetector<T> {
    fn drop(&mut self) {
        println!("DropDetector is being dropped!");
    }
}

#[derive(Debug)]
pub struct SearchAndReplaceEditingResponse {
    updated_code: String,
    response: String,
}

impl SearchAndReplaceEditingResponse {
    pub fn new(updated_code: String, response: String) -> Self {
        Self {
            updated_code,
            response,
        }
    }

    pub fn updated_code(&self) -> &str {
        &self.updated_code
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
    complete_file: String,
    extra_data: String,
    llm_properties: LLMProperties,
    new_symbols: Option<String>,
    instructions: String,
    root_request_id: String,
    symbol_identifier: SymbolIdentifier,
    edit_request_id: String,
    ui_sender: UnboundedSender<UIEventWithID>,
    user_context: Option<String>,
    editor_url: String,
    // use a is_warmup field
    is_warmup: bool,
}

impl SearchAndReplaceEditingRequest {
    pub fn new(
        fs_file_path: String,
        edit_range: Range,
        context_in_edit_selection: String,
        complete_file: String,
        extra_data: String,
        llm_properties: LLMProperties,
        new_symbols: Option<String>,
        instructions: String,
        root_request_id: String,
        symbol_identifier: SymbolIdentifier, // Unique identifier for the symbol being edited
        edit_request_id: String,
        ui_sender: UnboundedSender<UIEventWithID>,
        // Important: user_context provides essential information for the editing process
        user_context: Option<String>,
        // Indicates whether this is a warmup request to prepare the LLM
        editor_url: String,
        is_warmup: bool, // If true, this is a warmup request to initialize the LLM without performing actual edits
    ) -> Self {
        Self {
            fs_file_path,
            _edit_range: edit_range,
            context_in_edit_selection,
            complete_file,
            extra_data,
            llm_properties,
            new_symbols,
            instructions,
            root_request_id,
            symbol_identifier,
            edit_request_id,
            ui_sender,
            user_context,
            editor_url,
            is_warmup,
        }
    }
}

struct StreamedEditingForEditor {
    client: reqwest::Client,
}

impl StreamedEditingForEditor {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn send_edit_event(&self, editor_url: String, edit_event: EditedCodeStreamingRequest) {
        let editor_endpoint = editor_url + "/apply_edits_streamed";
        let _ = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&edit_event).expect("to work"))
            .send()
            .await;
    }
}

pub struct SearchAndReplaceEditing {
    llm_client: Arc<LLMBroker>,
    lsp_open_file: Arc<Box<dyn Tool + Send + Sync>>,
    file_locker: Arc<Mutex<HashMap<String, (String, Arc<Semaphore>)>>>,
    _fail_over_llm: LLMProperties,
}

impl SearchAndReplaceEditing {
    pub fn new(
        llm_client: Arc<LLMBroker>,
        fail_over_llm: LLMProperties,
        lsp_open_file: Arc<Box<dyn Tool + Send + Sync>>,
    ) -> Self {
        Self {
            llm_client,
            lsp_open_file,
            file_locker: Arc::new(Mutex::new(Default::default())),
            _fail_over_llm: fail_over_llm,
        }
    }

    fn system_message(&self) -> String {
        format!(r#"Act as an expert software developer.
Always use best practices when coding.
Respect and use existing conventions, libraries, etc that are already present in the code base.
You are diligent and tireless!
Write as little code as possible, opting for tiny, incremental changes. Add more code as last resort. Respond diligently to removing and editing code as well as adding.
The most important principle is to keep it simple. Always opt for the simplest, smallest changes.
You NEVER leave comments describing code without implementing it!
You always COMPLETELY IMPLEMENT the needed code!
You will be presented with a single file and the code which you can EDIT will be given in a <code_to_edit_section>
You will be also provided with some extra data, which contains various definitions of symbols which you can use to use the call the correct functions and re-use existing functionality in the code, this will be provided to you in <user_provided_context>
You are not to make changes in the <user_provided_context> ONLY EDIT the code in <code_to_edit_section>
Take requests for changes to the supplied code.
If the request is ambiguous, ask questions.

Always reply to the user in the same language they are using.

Once you understand the request you MUST:
1. Decide if you need to propose *SEARCH/REPLACE* edits to any files that haven't been added to the chat. You can create new files without asking. But if you need to propose edits to existing files not already added to the chat, you *MUST* tell the user their full path names and ask them to *add the files to the chat*. End your reply and wait for their approval. You can keep asking if you then decide you need to edit more files.
2. Describe each change with a *SEARCH/REPLACE block* per the examples below. All changes to files must use this *SEARCH/REPLACE block* format. ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!
3. If you do not need to make changes based on the user query, do not edit the code or generate any *SEARCH/REPLACE block*, leave the code as is.
4. Do not leave comments describing why a change should not be done or describing the functionality of the code, only use comments if the code has been functionally modified to do something else.

All changes to files must use the *SEARCH/REPLACE block* format.

# *SEARCH/REPLACE block* Rules:

Every *SEARCH/REPLACE block* must use this format:
1. The file path alone on a line, verbatim. No bold asterisks, no quotes around it, no escaping of characters, etc.
2. The opening fence and code language, eg: ```python
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
ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!"#).to_owned()
    }

    fn extra_data(&self, extra_data: &str) -> String {
        format!(
            r#"This is the extra data which you can use:
<extra_data>
{extra_data}
</extra_data>"#
        )
    }

    fn _above_selection(&self, above_selection: Option<&str>) -> Option<String> {
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

    fn _below_selection(&self, below_selection: Option<&str>) -> Option<String> {
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

    fn user_messages(&self, context: SearchAndReplaceEditingRequest) -> Vec<LLMClientMessage> {
        let mut messages = vec![];
        let user_context = context.user_context;
        if let Some(user_context) = user_context {
            let user_provided_context = LLMClientMessage::user(format!(
                r#"<user_provided_context>
{user_context}
</user_provided_context>
As a reminder, once you understand the request you MUST:
1. Decide if you need to propose *SEARCH/REPLACE* edits to any files that haven't been added to the chat. You can create new files without asking. But if you need to propose edits to existing files not already added to the chat, you *MUST* tell the user their full path names and ask them to *add the files to the chat*. End your reply and wait for their approval. You can keep asking if you then decide you need to edit more files.
2. Describe each change with a *SEARCH/REPLACE block* per the examples below. All changes to files must use this *SEARCH/REPLACE block* format. ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!
3. If you do not need to make changes based on the user query, do not edit the code or generate any *SEARCH/REPLACE block*, leave the code as is.
4. Do not leave comments describing why a change should not be done or describing the functionality of the code, only use comments if the code has been functionally modified to do something else.

All changes to files must use the *SEARCH/REPLACE block* format.

# *SEARCH/REPLACE block* Rules:

Every *SEARCH/REPLACE block* must use this format:
1. The file path alone on a line, verbatim. No bold asterisks, no quotes around it, no escaping of characters, etc.
2. The opening fence and code language, eg: ```python
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
ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!"#
            ))
            // double enforce the fact that we need replies in search and replace fashion
            // or we can also do many more few-shot requests
            .cache_point();
            messages.push(user_provided_context);
        }
        let extra_data = self.extra_data(&context.extra_data);
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
        user_message = user_message + &in_range + "\n";
        let instructions = if in_range.lines().into_iter().collect::<Vec<_>>().len() >= 1000 {
            let context_instructions = context.instructions;
            format!(
                r#"{context_instructions}
Think carefully since this is a long file where you have to make the changes"#
            )
        } else {
            context.instructions
        };
        let fs_file_path = context.fs_file_path;
        user_message = user_message
            + &format!(
                r#"Only edit the code in <code_to_edit_selection> my instructions are:
<user_instruction>
{instructions}
</user_instruction>

<fs_file_path>
{fs_file_path}
</fs_file_path>
"#
            );
        messages.push(LLMClientMessage::user(user_message));
        messages
    }

    fn example_messages(&self) -> Vec<LLMClientMessage> {
        vec![
            LLMClientMessage::user(r#"Change get_factorial() to use math.factorial"#.to_owned()),
            LLMClientMessage::assistant(
                r#"Here are the *SEARCH/REPLACE* blocks:

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
            )
            .cache_point(),
        ]
    }
}

#[async_trait]
impl Tool for SearchAndReplaceEditing {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.should_search_and_replace_editing()?;
        let is_warmup = context.is_warmup;
        let whole_file_context = context.complete_file.to_owned();
        let start_line = 0;
        let symbol_identifier = context.symbol_identifier.clone();
        let ui_sender = context.ui_sender.clone();
        let fs_file_path = context.fs_file_path.to_owned();
        let editor_url = context.editor_url.to_owned();
        let file_lock;
        {
            let cloned_file_locker = self.file_locker.clone();
            let mut file_locker = cloned_file_locker.lock().await;
            file_lock = if file_locker.contains_key(&fs_file_path) {
                let lock_acquired = file_locker
                    .get(&fs_file_path)
                    .expect("contains_key to work")
                    .clone();
                println!(
                    "lock_for_fs_file_path({})::lock_idx({})",
                    &fs_file_path, &lock_acquired.0
                );
                lock_acquired.1
            } else {
                let file_lock = Arc::new(Semaphore::new(1));
                let idx = uuid::Uuid::new_v4().to_string();
                file_locker.insert(fs_file_path.to_owned(), (idx.to_owned(), file_lock.clone()));
                println!(
                    "lock_for_fs_file_path::insert::fs_file_path({})::lock_idx({})",
                    fs_file_path, idx
                );
                file_lock
            };
        }
        let edit_request_id = context.edit_request_id.to_owned();
        let llm_properties = context.llm_properties.clone();
        let root_request_id = context.root_request_id.to_owned();
        let system_message = LLMClientMessage::system(self.system_message());
        let user_messages = self.user_messages(context);
        let example_messages = self.example_messages();
        let mut request = LLMClientCompletionRequest::new(
            llm_properties.llm().clone(),
            vec![system_message]
                .into_iter()
                .chain(example_messages)
                .chain(user_messages)
                .collect(),
            0.2,
            None,
        );
        if is_warmup {
            request = request.set_max_tokens(1);
        }
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let cloned_llm_client = self.llm_client.clone();
        let cloned_root_request_id = root_request_id.to_owned();
        let llm_response = tokio::spawn(async move {
            cloned_llm_client
                .stream_completion(
                    llm_properties.api_key().clone(),
                    request,
                    llm_properties.provider().clone(),
                    vec![
                        (
                            "event_type".to_owned(),
                            "search_and_replace_editing".to_owned(),
                        ),
                        ("root_id".to_owned(), cloned_root_request_id.to_owned()),
                    ]
                    .into_iter()
                    .collect(),
                    sender,
                )
                .await
        });

        let (edits_sender, mut edits_receiver) = tokio::sync::mpsc::unbounded_channel();
        // let (locks_sender, mut locks_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut search_and_replace_accumulator =
            SearchAndReplaceAccumulator::new(whole_file_context, start_line, edits_sender);

        // we want to figure out how poll the llm stream while locking up until the file is free
        // from the lock over here for the file path we are interested in
        let cloned_file_lock = file_lock.clone();
        let cloned_ui_sender = ui_sender.clone();
        let cloned_root_request_id = root_request_id.to_owned();
        let cloned_edit_request_id = edit_request_id.to_owned();
        let cloned_lsp_open_file = self.lsp_open_file.clone();
        let cloned_fs_file_path = fs_file_path.to_owned();
        let cloned_editor_url = editor_url.to_owned();

        let mut stream_answer = "".to_owned();

        let join_handle = tokio::spawn(async move {
            let file_lock = cloned_file_lock;
            let mut edit_lock = None;
            let _ui_sender = cloned_ui_sender.clone();
            let _root_request_id = cloned_root_request_id;
            let edit_request_id = cloned_edit_request_id;
            let lsp_open_file = cloned_lsp_open_file;
            let fs_file_path = cloned_fs_file_path;
            let editor_url = cloned_editor_url;
            let streamed_edit_client = StreamedEditingForEditor::new();
            // figure out what to do over here
            #[allow(irrefutable_let_patterns)]
            while let edits_response = edits_receiver.recv().await {
                // now over here we can manage the locks which we are getting and hold on to them for the while we are interested in
                // TODO(skcd): The lock needs to happen over here since we might
                // be processing the data in a stream so we want to hold onto it
                // for longer than required since we are getting the data in chunks
                // so we end up releasing very quickly
                match edits_response {
                    Some(EditDelta::EditLockAcquire(sender)) => {
                        edit_lock = Some(
                            file_lock
                                .clone()
                                .acquire_owned()
                                .await
                                .map(|data| DropDetector(data)),
                        );
                        // TODO(codestory): here we want to first get the lock for the file and re-read
                        // the contents for the file over here
                        let file_content = lsp_open_file
                            .invoke(ToolInput::OpenFile(OpenFileRequest::new(
                                fs_file_path.to_owned(),
                                editor_url.to_owned(),
                            )))
                            .await
                            .map(|output| output.get_file_open_response())
                            .ok()
                            .flatten();
                        if let Some(file_content) = file_content {
                            let _ = sender.send(Some(file_content.contents()));
                        } else {
                            let _ = sender.send(None);
                        }
                    }
                    Some(EditDelta::EditLockRelease) => {
                        let edit_lock_value = edit_lock;
                        edit_lock = None;
                        if let Some(Ok(edit_lock)) = edit_lock_value {
                            drop(DropDetector(edit_lock));
                        }
                    }
                    Some(EditDelta::EditStarted(range)) => {
                        streamed_edit_client
                            .send_edit_event(
                                editor_url.to_owned(),
                                EditedCodeStreamingRequest::start_edit(
                                    edit_request_id.to_owned(),
                                    range,
                                    fs_file_path.to_owned(),
                                ),
                            )
                            .await;
                        streamed_edit_client
                            .send_edit_event(
                                editor_url.to_owned(),
                                EditedCodeStreamingRequest::delta(
                                    edit_request_id.to_owned(),
                                    range,
                                    fs_file_path.to_owned(),
                                    "```\n".to_owned(),
                                ),
                            )
                            .await;
                    }
                    Some(EditDelta::EditDelta((range, delta))) => {
                        streamed_edit_client
                            .send_edit_event(
                                editor_url.to_owned(),
                                EditedCodeStreamingRequest::delta(
                                    edit_request_id.to_owned(),
                                    range,
                                    fs_file_path.to_owned(),
                                    delta,
                                ),
                            )
                            .await;
                    }
                    Some(EditDelta::EditEnd(range)) => {
                        streamed_edit_client
                            .send_edit_event(
                                editor_url.to_owned(),
                                EditedCodeStreamingRequest::delta(
                                    edit_request_id.to_owned(),
                                    range,
                                    fs_file_path.to_owned(),
                                    "\n```".to_owned(),
                                ),
                            )
                            .await;
                        streamed_edit_client
                            .send_edit_event(
                                editor_url.to_owned(),
                                EditedCodeStreamingRequest::end(
                                    edit_request_id.to_owned(),
                                    range,
                                    fs_file_path.to_owned(),
                                ),
                            )
                            .await;
                    }
                    Some(EditDelta::EndPollingStream) => {
                        break;
                    }
                    None => {
                        // println!("none_event_in_edit_delta::({})", &idx);
                    }
                }
            }
        });

        // over here we are getting the stream of deltas and also the final
        // answer which we are getting from the LLM
        // we want to process it in a fashion where we are consume the stream
        // and then return the answer while waiting on the future to finish

        // start consuming from the stream
        let mut delta_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
        while let Some(stream_msg) = delta_stream.next().await {
            let delta = stream_msg.delta();
            if let Some(delta) = delta {
                stream_answer.push_str(&delta);
                // we have some delta over here which we can process
                search_and_replace_accumulator
                    .add_delta(delta.to_owned())
                    .await;
                // send over the thinking as soon as we get a delta over here
                let _ = ui_sender.send(UIEventWithID::send_thinking_for_edit(
                    root_request_id.to_owned(),
                    symbol_identifier.clone(),
                    search_and_replace_accumulator.answer_to_show.to_owned(),
                    edit_request_id.to_owned(),
                ));
            }
        }

        // force the flush to happen over here
        search_and_replace_accumulator.process_answer().await;
        search_and_replace_accumulator.end_streaming().await;
        // we stop polling from the events stream once we are done with the llm response and the loop has finished
        let _ = join_handle.await;
        match llm_response.await {
            Ok(Ok(response)) => Ok(ToolOutput::search_and_replace_editing(
                SearchAndReplaceEditingResponse::new(
                    search_and_replace_accumulator.code_lines.join("\n"),
                    response,
                ),
            )),
            // wrong error over here but its fine for now
            _ => Err(ToolError::RetriesExhausted),
        }
    }
}

enum EditDelta {
    EditStarted(Range),
    EditDelta((Range, String)),
    EditEnd(Range),
    EditLockAcquire(tokio::sync::oneshot::Sender<Option<String>>),
    EditLockRelease,
    EndPollingStream,
}

#[derive(Debug, Clone)]
enum SearchBlockStatus {
    NoBlock,
    BlockStart,
    BlockAccumulate(String),
    BlockFound((String, Range)),
}

struct SearchAndReplaceAccumulator {
    code_lines: Vec<String>,
    start_line: usize,
    answer_up_until_now: String,
    answer_to_show: String,
    previous_answer_line_number: Option<usize>,
    search_block_status: SearchBlockStatus,
    updated_block: Option<String>,
    sender: UnboundedSender<EditDelta>,
}

impl SearchAndReplaceAccumulator {
    pub fn new(
        code_to_edit: String,
        start_line: usize,
        sender: UnboundedSender<EditDelta>,
    ) -> Self {
        Self {
            code_lines: code_to_edit
                .lines()
                .into_iter()
                .map(|line| line.to_owned())
                .collect::<Vec<_>>(),
            start_line,
            answer_up_until_now: "".to_owned(),
            answer_to_show: "".to_owned(),
            previous_answer_line_number: None,
            search_block_status: SearchBlockStatus::NoBlock,
            updated_block: None,
            sender,
        }
    }

    async fn end_streaming(&mut self) {
        let _ = self.sender.send(EditDelta::EndPollingStream);
    }

    async fn add_delta(&mut self, delta: String) {
        self.answer_up_until_now.push_str(&delta);
        self.process_answer().await;
        // check if we have a new search block starting here
    }

    async fn process_answer(&mut self) {
        let head = "<<<<<<< SEARCH";
        let divider = "=======";
        let updated = vec![">>>>>>> REPLACE", "======="];

        let line_number_to_process = get_last_newline_line_number(&self.answer_up_until_now);
        if line_number_to_process.is_none() {
            return;
        }
        let line_number_to_process_until = line_number_to_process.expect("to work") - 1;
        let answer_lines = self.answer_up_until_now.to_owned();
        let answer_lines = answer_lines.lines().into_iter().collect::<Vec<_>>();

        let start_index = self.previous_answer_line_number.map_or(0, |n| n + 1);

        for line_number in start_index..=line_number_to_process_until {
            self.previous_answer_line_number = Some(line_number);
            let answer_line_at_index = answer_lines[line_number];

            match self.search_block_status.clone() {
                SearchBlockStatus::NoBlock => {
                    if answer_line_at_index == head {
                        self.search_block_status = SearchBlockStatus::BlockStart;
                        let mut answer_lines = self
                            .answer_to_show
                            .lines()
                            .into_iter()
                            .map(|line| line.to_string())
                            .collect::<Vec<_>>();
                        answer_lines.push("Locating relevant snippet...".to_owned());
                        // at this point we will have the following:
                        // ```language
                        // Locating relevant snippet...
                        // we want to swap it to the following:
                        // Locating relevant snippet...
                        // ```language
                        let answer_lines_len = answer_lines.len();
                        let answer_lines_last = answer_lines[answer_lines_len - 1].to_owned();
                        let answer_lines_second_last =
                            answer_lines[answer_lines_len - 2].to_owned();
                        answer_lines[answer_lines_len - 1] = answer_lines_second_last;
                        answer_lines[answer_lines_len - 2] = answer_lines_last;
                        self.answer_to_show = answer_lines.join("\n");
                    } else {
                        if answer_line_at_index != "```" {
                            // add the line to our answer
                            let mut answer_lines = self
                                .answer_to_show
                                .lines()
                                .into_iter()
                                .map(|line| line.to_string())
                                .collect::<Vec<_>>();
                            answer_lines.push(answer_line_at_index.to_owned());
                            self.answer_to_show = answer_lines.join("\n");
                        }
                    }
                }
                SearchBlockStatus::BlockStart => {
                    self.search_block_status =
                        SearchBlockStatus::BlockAccumulate(answer_line_at_index.to_owned());
                    let mut answer_lines = self
                        .answer_to_show
                        .lines()
                        .into_iter()
                        .map(|line| line.to_string())
                        .collect::<Vec<_>>();
                    answer_lines.push(answer_line_at_index.to_owned());
                    answer_lines.push("```".to_owned());
                    self.answer_to_show = answer_lines.join("\n");
                }
                SearchBlockStatus::BlockAccumulate(accumulated) => {
                    if answer_line_at_index == divider {
                        let (sender, receiver) = tokio::sync::oneshot::channel();
                        let _result = self.sender.send(EditDelta::EditLockAcquire(sender));
                        let file_contents = receiver.await.ok().flatten();
                        let _time_now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("to work")
                            .as_millis();
                        if let Some(file_contents) = file_contents {
                            self.code_lines = file_contents
                                .lines()
                                .into_iter()
                                .map(|line| line.to_owned())
                                .collect::<Vec<_>>();
                        }
                        // and hold the lock for a while until we have the replace block
                        let range = get_range_for_search_block(
                            &self.code_lines.join("\n"),
                            self.start_line,
                            &accumulated,
                        );
                        match range {
                            Some(range) => {
                                self.search_block_status = SearchBlockStatus::BlockFound((
                                    accumulated.to_owned(),
                                    range.clone(),
                                ));
                                let _ = self.sender.send(EditDelta::EditStarted(range));
                                // If we have a range over here, we probably want to show it on the answer lines
                                // to do this: we need to do the following:
                                // - go back couple of steps here (or the line length of the accumulated block + 2 (for ```language and Locating relevant snippet...))
                                // - and the replace those lines with a generating code thingy over here instead
                                let accumualated_length =
                                    accumulated.lines().into_iter().collect::<Vec<_>>().len();
                                let mut answer_lines = self
                                    .answer_to_show
                                    .to_owned()
                                    .lines()
                                    .into_iter()
                                    .map(|answer_line| answer_line.to_owned())
                                    .collect::<Vec<_>>();
                                let answer_lines_len = answer_lines.len();
                                // we want to remove the first line in our answer which is
                                // locating relevant snippets
                                // then the ```{language}
                                // accumulated lines for the search block
                                // and the last ``` which we leave for rendering purposes
                                answer_lines.truncate(answer_lines_len - (accumualated_length + 3));
                                answer_lines.push("Generating code....".to_owned());
                                self.answer_to_show = answer_lines.join("\n");
                            }
                            None => {
                                // TODO(codestory): release the lock immediately
                                let _ = self.sender.send(EditDelta::EditLockRelease);

                                self.search_block_status = SearchBlockStatus::NoBlock;
                                // If we have a range over here, we probably want to show it on the answer lines
                                // to do this: we need to do the following:
                                // - go back couple of steps here (or the line length of the accumulated block + 3 (for ```language and Locating relevant snippet... and the last backticks which are present))
                                // - and the replace those lines with a "No snippet found in the codebase"
                                let accumualated_length =
                                    accumulated.lines().into_iter().collect::<Vec<_>>().len();
                                let mut answer_lines = self
                                    .answer_to_show
                                    .to_owned()
                                    .lines()
                                    .into_iter()
                                    .map(|answer_line| answer_line.to_owned())
                                    .collect::<Vec<_>>();
                                let answer_lines_len = answer_lines.len();
                                answer_lines.truncate(answer_lines_len - (accumualated_length + 3));
                                answer_lines
                                    .push("Failed to find relevant code snippet...".to_owned());
                                self.answer_to_show = answer_lines.join("\n");
                            }
                        };
                    } else {
                        self.search_block_status = SearchBlockStatus::BlockAccumulate(format!(
                            "{}\n{}",
                            accumulated, answer_line_at_index
                        ));
                        let mut answer_lines = self
                            .answer_to_show
                            .lines()
                            .into_iter()
                            .map(|line| line.to_string())
                            .collect::<Vec<_>>();
                        // remove the ``` which we added
                        answer_lines.pop();
                        answer_lines.push(answer_line_at_index.to_owned());
                        // add the ``` again
                        answer_lines.push("```".to_owned());
                        self.answer_to_show = answer_lines.join("\n");
                    }
                }
                SearchBlockStatus::BlockFound((_, block_range)) => {
                    if updated
                        .iter()
                        .any(|updated_trace| *updated_trace == answer_line_at_index)
                    {
                        self.search_block_status = SearchBlockStatus::NoBlock;
                        self.update_code_lines(&block_range);
                        let _ = self.sender.send(EditDelta::EditEnd(block_range.clone()));
                        // TODO(codestory): release the lock over here which we were holding on to
                        // since we are done editing the file for our section of the code
                        // this way we are sure to never lock up immediately
                        let _ = self.sender.send(EditDelta::EditLockRelease);

                        // remove the last line from the answer and instead put in edit completed
                        let mut answer_lines = self
                            .answer_to_show
                            .lines()
                            .into_iter()
                            .map(|line| line.to_owned())
                            .collect::<Vec<_>>();
                        answer_lines.pop();
                        answer_lines.push(format!(
                            "Edit completed: [L{}:{}]",
                            block_range.start_line(),
                            block_range.end_line()
                        ));
                        self.answer_to_show = answer_lines.join("\n");
                    } else {
                        self.update_block(answer_line_at_index, &block_range);
                    }
                }
            }
        }
    }

    fn update_code_lines(&mut self, block_range: &Range) {
        if let Some(updated_answer) = self.updated_block.clone() {
            let updated_range_start_line = block_range.start_line() - self.start_line;
            let updated_range_end_line = block_range.end_line() - self.start_line;
            let mut updated_code_lines = self.code_lines[..updated_range_start_line].join("\n");
            if updated_range_start_line != 0 {
                updated_code_lines.push('\n');
            }
            updated_code_lines.push_str(&updated_answer);
            updated_code_lines.push('\n');
            updated_code_lines
                .push_str(&self.code_lines[(updated_range_end_line + 1)..].join("\n"));
            self.code_lines = updated_code_lines
                .lines()
                .map(|line| line.to_owned())
                .collect();
        } else {
            let updated_range_start_line = block_range.start_line() - self.start_line;
            let updated_range_end_line = block_range.end_line() - self.start_line;
            let mut updated_code_lines = self.code_lines[..updated_range_start_line].join("\n");
            updated_code_lines
                .push_str(&self.code_lines[(updated_range_end_line + 1)..].join("\n"));
            self.code_lines = updated_code_lines
                .lines()
                .map(|line| line.to_owned())
                .collect();
        }
        self.updated_block = None;
    }

    fn update_block(&mut self, answer_line_at_index: &str, block_range: &Range) {
        if self.updated_block.is_none() {
            self.updated_block = Some(answer_line_at_index.to_owned());
            let _ = self.sender.send(EditDelta::EditDelta((
                block_range.clone(),
                answer_line_at_index.to_owned(),
            )));
        } else {
            self.updated_block = Some(
                self.updated_block.clone().expect("is_none to hold") + "\n" + answer_line_at_index,
            );
            let _ = self.sender.send(EditDelta::EditDelta((
                block_range.clone(),
                ("\n".to_owned() + answer_line_at_index).to_owned(),
            )));
        }
    }
}

/// Hels to get the last line number which has a \n
fn get_last_newline_line_number(s: &str) -> Option<usize> {
    s.rfind('\n')
        .map(|last_index| s[..=last_index].chars().filter(|&c| c == '\n').count())
}

fn get_range_for_search_block(
    code_to_look_at: &str,
    start_line: usize,
    search_block: &str,
) -> Option<Range> {
    let code_to_look_at_lines = code_to_look_at
        .lines()
        .into_iter()
        .enumerate()
        .map(|(idx, line)| (idx + start_line, line.to_owned()))
        .collect::<Vec<_>>();

    let search_block_lines = search_block.lines().into_iter().collect::<Vec<_>>();
    let search_block_len = search_block_lines.len();
    if code_to_look_at_lines.len() < search_block_len {
        // return early over here if we do not want to edit this
        return None;
    }
    for i in 0..=code_to_look_at_lines.len() - search_block_len {
        if code_to_look_at_lines[i..i + search_block_len]
            .iter()
            .map(|(_, content)| content)
            .collect::<Vec<_>>()
            == search_block_lines
        {
            // we have our answer over here, now return the range
            return Some(Range::new(
                Position::new(code_to_look_at_lines[i].0, 0, 0),
                Position::new(code_to_look_at_lines[i + search_block_len - 1].0, 0, 0),
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::agentic::tool::{
        errors::ToolError, input::ToolInput, lsp::open_file::OpenFileResponse, output::ToolOutput,
        r#type::Tool,
    };

    use super::SearchAndReplaceAccumulator;
    use async_trait::async_trait;

    struct CacheFileOutput {
        content: String,
    }

    #[async_trait]
    impl Tool for CacheFileOutput {
        async fn invoke(&self, _input: ToolInput) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput::file_open(OpenFileResponse::new(
                "something".to_owned(),
                self.content.to_owned(),
                true,
                "something".to_owned(),
            )))
        }
    }

    /// TODO(skcd): Broken test here to debug multiple search and replace blocks being
    /// part of the same edit
    #[tokio::test]
    async fn test_multiple_search_and_edit_blocks() {
        let input_data = r#"impl LLMClientMessage {
    pub async fn new(role: LLMClientRole, message: String) -> Self {
        Self {
            role,
            message,
            function_call: None,
            function_return: None,
        }
    }

    pub fn concat_message(&mut self, message: &str) {
        self.message = self.message.to_owned() + "\n" + message;
    }

    pub fn concat(self, other: Self) -> Self {
        // We are going to concatenate the 2 llm client messages togehter, at this moment
        // we are just gonig to join the message with a \n
        Self {
            role: self.role,
            message: self.message + "\n" + &other.message,
            function_call: match self.function_call {
                Some(function_call) => Some(function_call),
                None => other.function_call,
            },
            function_return: match other.function_return {
                Some(function_return) => Some(function_return),
                None => self.function_return,
            },
        }
    }

    pub fn function_call(name: String, arguments: String) -> Self {
        Self {
            role: LLMClientRole::Assistant,
            message: "".to_owned(),
            function_call: Some(LLMClientMessageFunctionCall { name, arguments }),
            function_return: None,
        }
    }

    pub fn function_return(name: String, content: String) -> Self {
        Self {
            role: LLMClientRole::Function,
            message: "".to_owned(),
            function_call: None,
            function_return: Some(LLMClientMessageFunctionReturn { name, content }),
        }
    }

    pub fn user(message: String) -> Self {
        Self::new(LLMClientRole::User, message)
    }

    pub fn assistant(message: String) -> Self {
        Self::new(LLMClientRole::Assistant, message)
    }

    pub fn system(message: String) -> Self {
        Self::new(LLMClientRole::System, message)
    }

    pub fn content(&self) -> &str {
        &self.message
    }

    pub fn set_empty_content(&mut self) {
        self.message =
            "empty message found here, possibly an error but keep following the conversation"
                .to_owned();
    }

    pub fn function(message: String) -> Self {
        Self::new(LLMClientRole::Function, message)
    }

    pub fn role(&self) -> &LLMClientRole {
        &self.role
    }

    pub fn get_function_call(&self) -> Option<&LLMClientMessageFunctionCall> {
        self.function_call.as_ref()
    }

    pub fn get_function_return(&self) -> Option<&LLMClientMessageFunctionReturn> {
        self.function_return.as_ref()
    }
}"#;
        let edits = r#"/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
```rust
<<<<<<< SEARCH
    pub fn concat(self, other: Self) -> Self {
        // We are going to concatenate the 2 llm client messages togehter, at this moment
        // we are just gonig to join the message with a \n
        Self {
            role: self.role,
            message: self.message + "\n" + &other.message,
            function_call: match self.function_call {
                Some(function_call) => Some(function_call),
                None => other.function_call,
            },
            function_return: match other.function_return {
                Some(function_return) => Some(function_return),
                None => self.function_return,
            },
        }
    }

    pub fn function_call(name: String, arguments: String) -> Self {
        Self {
            role: LLMClientRole::Assistant,
            message: "".to_owned(),
            function_call: Some(LLMClientMessageFunctionCall { name, arguments }),
            function_return: None,
        }
    }

    pub fn function_return(name: String, content: String) -> Self {
        Self {
            role: LLMClientRole::Function,
            message: "".to_owned(),
            function_call: None,
            function_return: Some(LLMClientMessageFunctionReturn { name, content }),
        }
    }

    pub fn user(message: String) -> Self {
        Self::new(LLMClientRole::User, message)
    }

    pub fn assistant(message: String) -> Self {
        Self::new(LLMClientRole::Assistant, message)
    }

    pub fn system(message: String) -> Self {
        Self::new(LLMClientRole::System, message)
    }
=======
    pub fn concat(self, other: Self) -> impl Future<Output = Self> {
        async move {
            // We are going to concatenate the 2 llm client messages togehter, at this moment
            // we are just gonig to join the message with a \n
            Self {
                role: self.role,
                message: self.message + "\n" + &other.message,
                function_call: match self.function_call {
                    Some(function_call) => Some(function_call),
                    None => other.function_call,
                },
                function_return: match other.function_return {
                    Some(function_return) => Some(function_return),
                    None => self.function_return,
                },
            }
        }
    }

    pub fn function_call(name: String, arguments: String) -> impl Future<Output = Self> {
        async move {
            Self {
                role: LLMClientRole::Assistant,
                message: "".to_owned(),
                function_call: Some(LLMClientMessageFunctionCall { name, arguments }),
                function_return: None,
            }
        }
    }

    pub fn function_return(name: String, content: String) -> impl Future<Output = Self> {
        async move {
            Self {
                role: LLMClientRole::Function,
                message: "".to_owned(),
                function_call: None,
                function_return: Some(LLMClientMessageFunctionReturn { name, content }),
            }
        }
    }

    pub fn user(message: String) -> impl Future<Output = Self> {
        Self::new(LLMClientRole::User, message)
    }

    pub fn assistant(message: String) -> impl Future<Output = Self> {
        Self::new(LLMClientRole::Assistant, message)
    }

    pub fn system(message: String) -> impl Future<Output = Self> {
        Self::new(LLMClientRole::System, message)
    }
>>>>>>> REPLACE
```

/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
```rust
<<<<<<< SEARCH
    pub fn function(message: String) -> Self {
        Self::new(LLMClientRole::Function, message)
    }
=======
    pub fn function(message: String) -> impl Future<Output = Self> {
        Self::new(LLMClientRole::Function, message)
    }
>>>>>>> REPLACE
```"#;

        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut search_and_replace_accumulator =
            SearchAndReplaceAccumulator::new(input_data.to_owned(), 0, sender);
        search_and_replace_accumulator
            .add_delta(edits.to_owned())
            .await;
        let final_lines = search_and_replace_accumulator.code_lines.join("\n");
        assert_eq!(
            final_lines,
            r#"impl LLMClientMessage {
    pub async fn new(role: LLMClientRole, message: String) -> Self {
        Self {
            role,
            message,
            function_call: None,
            function_return: None,
        }
    }

    pub fn concat_message(&mut self, message: &str) {
        self.message = self.message.to_owned() + "\n" + message;
    }

    pub fn concat(self, other: Self) -> impl Future<Output = Self> {
        async move {
            // We are going to concatenate the 2 llm client messages togehter, at this moment
            // we are just gonig to join the message with a \n
            Self {
                role: self.role,
                message: self.message + "\n" + &other.message,
                function_call: match self.function_call {
                    Some(function_call) => Some(function_call),
                    None => other.function_call,
                },
                function_return: match other.function_return {
                    Some(function_return) => Some(function_return),
                    None => self.function_return,
                },
            }
        }
    }

    pub fn function_call(name: String, arguments: String) -> impl Future<Output = Self> {
        async move {
            Self {
                role: LLMClientRole::Assistant,
                message: "".to_owned(),
                function_call: Some(LLMClientMessageFunctionCall { name, arguments }),
                function_return: None,
            }
        }
    }

    pub fn function_return(name: String, content: String) -> impl Future<Output = Self> {
        async move {
            Self {
                role: LLMClientRole::Function,
                message: "".to_owned(),
                function_call: None,
                function_return: Some(LLMClientMessageFunctionReturn { name, content }),
            }
        }
    }

    pub fn user(message: String) -> impl Future<Output = Self> {
        Self::new(LLMClientRole::User, message)
    }

    pub fn assistant(message: String) -> impl Future<Output = Self> {
        Self::new(LLMClientRole::Assistant, message)
    }

    pub fn system(message: String) -> impl Future<Output = Self> {
        Self::new(LLMClientRole::System, message)
    }

    pub fn content(&self) -> &str {
        &self.message
    }

    pub fn set_empty_content(&mut self) {
        self.message =
            "empty message found here, possibly an error but keep following the conversation"
                .to_owned();
    }

    pub fn function(message: String) -> impl Future<Output = Self> {
        Self::new(LLMClientRole::Function, message)
    }

    pub fn role(&self) -> &LLMClientRole {
        &self.role
    }

    pub fn get_function_call(&self) -> Option<&LLMClientMessageFunctionCall> {
        self.function_call.as_ref()
    }

    pub fn get_function_return(&self) -> Option<&LLMClientMessageFunctionReturn> {
        self.function_return.as_ref()
    }
}"#
        );
    }

    #[tokio::test]
    async fn test_search_and_replace_removing_code() {
        let original_code = r#"impl SymbolToEdit {
    pub fn new(
        symbol_name: String,
        range: Range,
        fs_file_path: String,
        instructions: Vec<String>,
        outline: bool,
        is_new: bool,
        is_full_edit: bool,
        original_user_query: String,
        symbol_edited_list: Option<Vec<SymbolEditedItem>>,
    ) -> Self {
        Self {
            symbol_name,
            range,
            outline,
            fs_file_path,
            instructions,
            is_new,
            is_full_edit,
            original_user_query,
            symbol_edited_list,
        }
    }

    pub fn symbol_edited_list(&self) -> Option<Vec<SymbolEditedItem>> {
        self.symbol_edited_list.clone()
    }

    pub fn original_user_query(&self) -> &str {
        &self.original_user_query
    }

    pub fn is_full_edit(&self) -> bool {
        self.is_full_edit
    }

    pub fn set_fs_file_path(&mut self, fs_file_path: String) {
        self.fs_file_path = fs_file_path;
    }

    pub fn set_range(&mut self, range: Range) {
        self.range = range;
    }

    pub fn is_new(&self) -> bool {
        self.is_new.clone()
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn is_outline(&self) -> bool {
        self.outline
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub fn instructions(&self) -> &[String] {
        self.instructions.as_slice()
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }
}"#;
        let edits = r#"/Users/zi/codestory/testing/sidecar/sidecar/src/agentic/symbol/events/edit.rs
```rust
<<<<<<< SEARCH
impl SymbolToEdit {
    pub fn new(
        symbol_name: String,
        range: Range,
        fs_file_path: String,
        instructions: Vec<String>,
        outline: bool,
        is_new: bool,
        is_full_edit: bool,
        original_user_query: String,
        symbol_edited_list: Option<Vec<SymbolEditedItem>>,
    ) -> Self {
        Self {
            symbol_name,
            range,
            outline,
            fs_file_path,
            instructions,
            is_new,
            is_full_edit,
            original_user_query,
            symbol_edited_list,
        }
    }
=======
impl SymbolToEdit {
    pub fn new(
        symbol_name: String,
        range: Range,
        fs_file_path: String,
        instructions: Vec<String>,
        is_new: bool,
        is_full_edit: bool,
        original_user_query: String,
        symbol_edited_list: Option<Vec<SymbolEditedItem>>,
    ) -> Self {
        Self {
            symbol_name,
            range,
            fs_file_path,
            instructions,
            is_new,
            is_full_edit,
            original_user_query,
            symbol_edited_list,
        }
    }
>>>>>>> REPLACE
```

/Users/zi/codestory/testing/sidecar/sidecar/src/agentic/symbol/events/edit.rs
```rust
<<<<<<< SEARCH
    pub fn is_outline(&self) -> bool {
        self.outline
    }

=======
>>>>>>> REPLACE
```"#;
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut search_and_replace_accumulator =
            SearchAndReplaceAccumulator::new(original_code.to_owned(), 0, sender);
        search_and_replace_accumulator
            .add_delta(edits.to_owned())
            .await;
        let final_code = search_and_replace_accumulator.code_lines.join("\n");
        assert_eq!(
            final_code,
            r#"impl SymbolToEdit {
    pub fn new(
        symbol_name: String,
        range: Range,
        fs_file_path: String,
        instructions: Vec<String>,
        is_new: bool,
        is_full_edit: bool,
        original_user_query: String,
        symbol_edited_list: Option<Vec<SymbolEditedItem>>,
    ) -> Self {
        Self {
            symbol_name,
            range,
            fs_file_path,
            instructions,
            is_new,
            is_full_edit,
            original_user_query,
            symbol_edited_list,
        }
    }

    pub fn symbol_edited_list(&self) -> Option<Vec<SymbolEditedItem>> {
        self.symbol_edited_list.clone()
    }

    pub fn original_user_query(&self) -> &str {
        &self.original_user_query
    }

    pub fn is_full_edit(&self) -> bool {
        self.is_full_edit
    }

    pub fn set_fs_file_path(&mut self, fs_file_path: String) {
        self.fs_file_path = fs_file_path;
    }

    pub fn set_range(&mut self, range: Range) {
        self.range = range;
    }

    pub fn is_new(&self) -> bool {
        self.is_new.clone()
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub fn instructions(&self) -> &[String] {
        self.instructions.as_slice()
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }
}"#
        );
    }

    #[tokio::test]
    async fn test_with_broken_replace_block() {
        let code = r#"something
interesting
something_else
blahblah"#;
        let edits = r#"```
<<<<<<< SEARCH
something_else
blahblah
=======
blahblah2
=======
```"#;
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut search_and_replace_accumulator =
            SearchAndReplaceAccumulator::new(code.to_owned(), 0, sender);
        search_and_replace_accumulator
            .add_delta(edits.to_owned())
            .await;
        let final_code = search_and_replace_accumulator.code_lines.join("\n");
        assert_eq!(
            final_code,
            r#"something
interesting
blahblah2"#
        );
    }
}
