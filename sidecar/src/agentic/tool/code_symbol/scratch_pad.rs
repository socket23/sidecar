//! The scratchpad agent and the prompts for it
//! We are still not sure what this will look like, so consider everything over
//! here to be best effort

use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys},
};

use crate::agentic::{
    symbol::{
        identifier::SymbolIdentifier,
        ui_event::{EditedCodeStreamingRequest, UIEventWithID},
    },
    tool::{
        code_edit::search_and_replace::{
            EditDelta, SearchAndReplaceAccumulator, SearchAndReplaceEditingResponse,
            StreamedEditingForEditor,
        },
        errors::ToolError,
        input::ToolInput,
        output::ToolOutput,
        r#type::Tool,
    },
};

pub struct ScratchPadAgentBroker {
    llm_client: Arc<LLMBroker>,
}

impl ScratchPadAgentBroker {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }
}

#[derive(Debug, Clone)]
pub struct ScratchPadAgentHumanMessage {
    user_code_context: String,
    user_context_files: Vec<String>,
    query: String,
}

impl ScratchPadAgentHumanMessage {
    pub fn new(user_code_context: String, user_context_files: Vec<String>, query: String) -> Self {
        Self {
            user_code_context,
            user_context_files,
            query,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScratchPadAgentEdits {
    edits_made: Vec<String>,
    user_request: String,
}

#[derive(Debug, Clone)]
pub struct ScratchPadAgentEditorSignal {}

#[derive(Debug, Clone)]
pub enum ScratchPadAgentInputType {
    UserMessage(ScratchPadAgentHumanMessage),
    EditsMade(ScratchPadAgentEdits),
    EditorSignal(ScratchPadAgentEditorSignal),
    CacheWarmup,
}

impl ScratchPadAgentInputType {
    fn is_cache_warmup(&self) -> bool {
        matches!(self, Self::CacheWarmup)
    }

    fn to_string(self) -> String {
        match self {
            Self::UserMessage(user_message) => {
                let files = user_message.user_context_files.join("\n");
                let user_query = user_message.query;
                let user_context = user_message.user_code_context;
                format!(
                    r#"I am looking at the following files
<files>
{files}
</files>

The code which I want to edit:
<code_in_selection>
{user_context}
</code_in_selection>

The changes I intend to do:
<query>
{user_query}
</query>"#
                )
            }
            Self::EditsMade(edits_made) => {
                let user_query = edits_made.user_request;
                let edits_made = edits_made.edits_made.join("\n");
                format!(
                    r#"I have made the following changes:
<changes>
{edits_made}
</changes>

and my intention was:
<query>
{user_query}
</query>"#
                )
            }
            Self::EditorSignal(_editor_signal) => "".to_owned(),
            Self::CacheWarmup => "".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScratchPadAgentInput {
    files_context: Vec<String>,
    extra_context: String,
    input_event: ScratchPadAgentInputType,
    scratch_pad_content: String,
    scratch_pad_path: String,
    root_request_id: String,
    ui_sender: UnboundedSender<UIEventWithID>,
    editor_url: String,
}

impl ScratchPadAgentInput {
    pub fn new(
        files_context: Vec<String>,
        extra_context: String,
        input_event: ScratchPadAgentInputType,
        scratch_pad_content: String,
        scratch_pad_path: String,
        root_request_id: String,
        ui_sender: UnboundedSender<UIEventWithID>,
        editor_url: String,
    ) -> Self {
        Self {
            files_context,
            extra_context,
            input_event,
            scratch_pad_content,
            scratch_pad_path,
            root_request_id,
            ui_sender,
            editor_url,
        }
    }
}

struct ScratchPadAgentUserMessage {
    user_messages: Vec<LLMClientMessage>,
    is_cache_warmup: bool,
    scratch_pad_path: String,
    root_request_id: String,
    scratch_pad_content: String,
}

impl ScratchPadAgentBroker {
    fn system_message(&self) -> String {
        format!(
            r#"Act as an expert software engineer.
You are going to act as a second pair of eyes and brain for the developer working in a code editor.
Your task is to keep an eye on everything happening in the editor and come up with INSIGHTS and NEXT STEPS to help the user.
You will be given a scratchpad which you can use to record your work and thought process.
The scratchpad might be already populated with your thoughts from before.

The scratchpad is a special place structured as following:
<files_visible>
</files_visible>
<thinking>
</thinking>
<tasks>
</tasks>
<insights>
</insights>
<next_steps>
</next_steps>

You are free to use the scratchpad as your notebook where you can record your work.
We explain each section of the scratchpad below:
- <files_visible>
These are the files which are visible to you in the editor, if you want to open new files or ask for more information please use the <next_steps> section and state the WHY always
- <thinking>
You can use this to record your running thoughts, any progress which the user has made, this is space for your inner monologue
- <tasks>
These are the tasks which you are working on, make sure you mark a task which you are working on as [in_progress]. Keep this strucutred as a list (using -) and try to not repeat the same task again.
The developer also sees this and decides what they want to do next
- <insights>
The insights is a very special place where you can store new information you are learning. The information you write over here can be available to you in the future, so make sure you come up with genuine and innovative insights which will help you later.
- <next_steps>
The next steps over here reflect what you think we should do next after making progress on a task or based on some signal from the editor, developer or any other tooling.
You have to make sure your <next_steps> are grouned in the files which are open and not anywhere else.

The different kind of signals which you get are of the following type:
- The user might have asked you for a question about some portion of the code.
- The user intends to edit some part of the codebase and they are telling you what they plan on doing, you should not suggest the edits since they will be done by the user, your job is to just observer the intention and help the developer understand if they missed anything.
- The edits have been made and now you can learn something new from it, this will be your INSIGHT.
- The edits which have been made could lead to additional change in the current file or files which are open in the editor.
- The editor has a language server running which generates diagnostic signals, its really important that you make sure to suggest edits for these diagnostics.

Your scratchpad is a special place because the developer is also looking at it to inform themselves about the changes made to the codebase, so be concise and insightful in your scratchpad. Remember the developer trusts you a lot!

When you get a signal either from the developer or from the editor you must update the scratchpad, remember the developer is also using to keep an eye on the progress so be the most helpful pair-programmer you can be!
The edits made to the scratchpad should be made in with SEARCH and REPLACE type of edits, the user message will explain to you how to do that, follow it to the letter and make no mistakes in the format."#
        )
    }

    fn user_message(&self, input: ScratchPadAgentInput) -> ScratchPadAgentUserMessage {
        let files_context = input.files_context.join("\n");
        let extra_context = input.extra_context;
        let event_type = input.input_event;
        let scratch_pad_content = input.scratch_pad_content;
        let scratch_pad_path = input.scratch_pad_path;
        let root_request_id = input.root_request_id;
        let is_cache_warmup = event_type.is_cache_warmup();
        let context_message = LLMClientMessage::user(format!(
            r#"I am providing you the files you asked for along with some extra context
<files_context>
{files_context}
</files_context>

<extra_context>
{extra_context}
</extra_context>

This is what I see in the scratchpad
{scratch_pad_content}"#
        ))
        .cache_point();
        let acknowledgment_message = LLMClientMessage::assistant("Thank you for providing me the additional context, I will keep this in mind when updating the scratchpad".to_owned()).cache_point();
        let user_message = if is_cache_warmup {
            event_type.to_string()
        } else {
            let event_type_str = event_type.to_string();
            format!(
                r#"
{event_type_str}

I will also explain to you how to use the *SEARCH/REPLACE* edits style:
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
ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!

I am also showing you an example:
Change get_factorial() to use math.factorial

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
```

Your goal is to update the scratch-pad and make sure to not forget that goal"#
            )
        };
        ScratchPadAgentUserMessage {
            user_messages: vec![
                context_message,
                acknowledgment_message,
                LLMClientMessage::user(user_message),
            ],
            is_cache_warmup,
            scratch_pad_path,
            root_request_id,
            scratch_pad_content,
        }
    }
}

#[async_trait]
impl Tool for ScratchPadAgentBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        // figure out what to do over here
        println!("scratch_pad_agent_broker::invoked");
        let context = input.should_scratch_pad_input()?;
        let ui_sender = context.ui_sender.clone();
        let fs_file_path = context.scratch_pad_path.to_owned();
        let editor_url = context.editor_url.to_owned();
        let system_message = LLMClientMessage::system(self.system_message());
        let user_messages_context = self.user_message(context);
        let is_cache_warmup = user_messages_context.is_cache_warmup;
        let user_messages = user_messages_context.user_messages;
        let root_request_id = user_messages_context.root_request_id;
        let scratch_pad_content = user_messages_context.scratch_pad_content;
        let mut request = LLMClientCompletionRequest::new(
            LLMType::ClaudeSonnet,
            vec![system_message]
                .into_iter()
                .chain(user_messages)
                .collect::<Vec<_>>(),
            0.2,
            None,
        );
        if is_cache_warmup {
            request = request.set_max_tokens(1);
        }
        let anthropic_api_key = "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".to_owned();
        let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key));
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let cloned_llm_client = self.llm_client.clone();
        let cloned_root_request_id = root_request_id.to_owned();
        let llm_response = tokio::spawn(async move {
            cloned_llm_client
                .stream_completion(
                    api_key,
                    request,
                    LLMProvider::Anthropic,
                    vec![
                        ("root_id".to_owned(), cloned_root_request_id),
                        ("event_type".to_owned(), "scratch_pad_agent".to_owned()),
                    ]
                    .into_iter()
                    .collect(),
                    sender,
                )
                .await
        });
        if is_cache_warmup {
            println!("scratch_pad_agent::cache_warmup::skipping_early");
            return Ok(ToolOutput::SearchAndReplaceEditing(
                SearchAndReplaceEditingResponse::new("".to_owned(), "".to_owned()),
            ));
        }

        let (edits_sender, mut edits_receiver) = tokio::sync::mpsc::unbounded_channel();
        // let (locks_sender, mut locks_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut search_and_replace_accumulator =
            SearchAndReplaceAccumulator::new(scratch_pad_content, 0, edits_sender);

        // we want to figure out how poll the llm stream while locking up until the file is free
        // from the lock over here for the file path we are interested in
        let cloned_ui_sender = ui_sender.clone();
        let cloned_root_request_id = root_request_id.to_owned();
        let edit_request_id = uuid::Uuid::new_v4().to_string();
        let cloned_edit_request_id = edit_request_id.to_owned();
        let cloned_fs_file_path = fs_file_path.to_owned();
        let cloned_editor_url = editor_url.to_owned();

        let mut stream_answer = "".to_owned();

        let join_handle = tokio::spawn(async move {
            let _ui_sender = cloned_ui_sender.clone();
            let _root_request_id = cloned_root_request_id;
            let edit_request_id = cloned_edit_request_id;
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
                        let _ = sender.send(None);
                    }
                    Some(EditDelta::EditLockRelease) => {
                        // doing nothing for the lock over here
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
                    SymbolIdentifier::with_file_path(&fs_file_path, &fs_file_path),
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
        println!("scratch_pad_agent_broker::finished");
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
