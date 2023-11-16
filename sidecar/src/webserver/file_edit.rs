use std::path::PathBuf;
use std::sync::Arc;

use axum::response::{sse, IntoResponse, Sse};
use axum::{Extension, Json};
use difftastic::LineInformation;
use futures::StreamExt;
use serde_json::json;

use crate::agent::llm_funcs::LlmClient;
use crate::agent::prompts::diff_accept_prompt;
use crate::agent::{llm_funcs, prompts};
use crate::application::application::Application;
use crate::chunking::languages::TSLanguageParsing;
use crate::chunking::text_document::{Position, Range};

use super::types::{json, ApiResponse, Result};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditFileRequest {
    pub file_path: String,
    pub file_content: String,
    pub new_content: String,
    pub language: String,
    pub user_query: String,
    pub session_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EditFileResponse {
    Message {
        message: String,
    },
    Action {
        action: DiffActionResponse,
        range: Range,
        content: String,
        previous_content: String,
    },
    TextEdit {
        range: Range,
        content: String,
    },
    Status {
        session_id: String,
        status: String,
    },
}

impl ApiResponse for EditFileResponse {}

pub async fn file_edit(
    Extension(app): Extension<Application>,
    Json(EditFileRequest {
        file_path,
        file_content,
        language,
        new_content,
        user_query,
        session_id,
    }): Json<EditFileRequest>,
) -> Result<impl IntoResponse> {
    // Here we have to first check if the new content is tree-sitter valid, if
    // thats the case only then can we apply it to the file
    // First we check if the output generated is valid by itself, if it is then
    // we can think about applying the changes to the file
    let llm_client = Arc::new(LlmClient::codestory_infra(app.posthog_client.clone()));
    let file_diff_content = generate_file_diff(
        &file_content,
        &file_path,
        &new_content,
        &language,
        app.language_parsing.clone(),
    )
    .await;
    if let None = file_diff_content {
        let cloned_session_id = session_id.clone();
        let init_stream = futures::stream::once(async move {
            Ok(sse::Event::default()
                .json_data(EditFileResponse::Status {
                    session_id: cloned_session_id,
                    status: "started".to_owned(),
                })
                // This should never happen, so we force an unwrap.
                .expect("failed to serialize initialization object"))
        });
        let message_stream = futures::stream::once(async move {
            Ok(sse::Event::default()
                .json_data(EditFileResponse::Message {
                    message: "Cannot apply the diff to the file".to_owned(),
                })
                // This should never happen, so we force an unwrap.
                .expect("failed to serialize initialization object"))
        });
        let done_stream = futures::stream::once(async move {
            Ok(sse::Event::default()
                .json_data(EditFileResponse::Status {
                    session_id,
                    status: "done".to_owned(),
                })
                .expect("failed to send done object"))
        });
        let stream: Result<
            Sse<
                std::pin::Pin<
                    Box<dyn tokio_stream::Stream<Item = anyhow::Result<sse::Event>> + Send>,
                >,
            >,
        > = Ok(Sse::new(Box::pin(
            init_stream.chain(message_stream).chain(done_stream),
        )));
        stream
    } else {
        // After generating the git diff we want to send back the responses to the
        // user depending on what edit information we get, we can stream this to the
        // user so they know the agent is working on some action and it will show up
        // as edits on the editor
        let result = process_file_lines_to_gpt(
            file_diff_content.unwrap(),
            user_query,
            session_id,
            llm_client,
        )
        .await;
        result
    }
}

pub async fn generate_file_diff(
    file_content: &str,
    file_path: &str,
    new_content: &str,
    language: &str,
    language_parsing: Arc<TSLanguageParsing>,
) -> Option<Vec<String>> {
    // First we will check with the language parsing if this is a valid tree
    // which we can apply to the edit
    let language_parser = language_parsing.for_lang(language);
    if language_parser.is_none() {
        return None;
    }
    let language_parser = language_parser.unwrap();
    let validity = language_parser.is_valid_code(file_content);
    if !validity {
        return None;
    }
    // we can get the extension from the file path
    let file_extension = PathBuf::from(file_path)
        .extension()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    // Cool so the tree is valid, then we can go about generating the diff tree now
    let difftastic_output =
        difftastic::generate_sidecar_diff(file_content, new_content, &format!(".{file_extension}"));
    // sanity check here if this is a valid tree
    let diff_file = parse_difftastic_output(
        file_content.to_owned(),
        new_content.to_owned(),
        difftastic_output.0,
        difftastic_output.1,
    )
    .await;
    Some(diff_file)
}

fn get_content_from_file_line_information(
    content: &str,
    line_information: &LineInformation,
) -> Option<String> {
    let lines: Vec<String> = content
        .lines()
        .into_iter()
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();
    let line_number = line_information.get_line_number();
    dbg!(line_number);
    match line_number {
        Some(line_number) => Some(lines[line_number].to_owned()),
        None => None,
    }
}

// Here we will first parse the llm output and get the left and right links
async fn parse_difftastic_output(
    left: String,
    right: String,
    left_lines_information: Vec<LineInformation>,
    right_lines_information: Vec<LineInformation>,
) -> Vec<String> {
    assert_eq!(left_lines_information.len(), right_lines_information.len());
    let mut iteration_index = 0;
    let left_lines_limit = left_lines_information.len();
    let mut final_lines_file: Vec<String> = vec![];
    // Remember: left is our main file and right is the diff which the LLM has
    // generated
    while iteration_index < left_lines_limit {
        // dbg!("iterating loop break, iterating again");
        loop {
            // dbg!("loop iteration", iteration_index);
            if iteration_index >= left_lines_limit {
                break;
            }
            // Now we will here greedily try to insert the markers for git and then
            let left_content_now_maybe = left_lines_information[iteration_index];
            if iteration_index >= right_lines_information.len() {
                // empty the left side to the final lines
                loop {
                    let left_content_now = left_lines_information[iteration_index];
                    let content = get_content_from_file_line_information(&left, &left_content_now);
                    match content {
                        Some(content) => {
                            final_lines_file.push(content);
                        }
                        None => {}
                    }
                    iteration_index = iteration_index + 1;
                    if iteration_index >= left_lines_information.len() {
                        break;
                    }
                }
            }
            let right_content_now_maybe = right_lines_information[iteration_index];
            // we have content on the left but nothing on the right, so we keep going for as long
            // as possible we have content
            if left_content_now_maybe.present() && right_content_now_maybe.not_present() {
                // Let's get the color of the left side
                // we will always have a left color ALWAYS and it will be RED or false
                let content =
                    get_content_from_file_line_information(&left, &left_content_now_maybe);
                match content {
                    Some(content) => {
                        final_lines_file.push(content);
                    }
                    None => {}
                }
                // Now we can start going down on left and right, if we keep getting
                // right None as usual..
                loop {
                    iteration_index = iteration_index + 1;
                    if left_lines_information.len() >= iteration_index {
                        break;
                    }
                    if right_lines_information.len() <= iteration_index {
                        // If we are here, we have to collect the rest of the lines
                        // in the right and call it a day
                        loop {
                            let left_content_now_maybe = left_lines_information[iteration_index];
                            let content = get_content_from_file_line_information(
                                &left,
                                &left_content_now_maybe,
                            );
                            match content {
                                Some(content) => {
                                    final_lines_file.push(content);
                                }
                                None => {}
                            }
                            iteration_index = iteration_index + 1;
                            if iteration_index >= left_lines_information.len() {
                                break;
                            }
                        }
                        break;
                    }
                    // otherwise we want to keep checking the lines after this
                    let left_content_now_maybe = left_lines_information[iteration_index];
                    let right_content_now_maybe = right_lines_information[iteration_index];
                    if !(left_content_now_maybe.present() && right_content_now_maybe.not_present())
                    {
                        // we are not in the same style as before, so we break it
                        break;
                    } else {
                        let content =
                            get_content_from_file_line_information(&left, &left_content_now_maybe);
                        match content {
                            Some(content) => {
                                final_lines_file.push(content);
                            }
                            None => {}
                        }
                    }
                }
                break;
            }
            // we have some content on the right but nothing ont he left
            if left_content_now_maybe.not_present() && right_content_now_maybe.present() {
                // Now we are in a state where we can be sure that on the right
                // we have a GREEN and nothing on the left side, cause that's
                // the only case where its possible
                let content =
                    get_content_from_file_line_information(&right, &right_content_now_maybe);
                match content {
                    Some(content) => {
                        final_lines_file.push(content);
                    }
                    None => {}
                }
                // Now we start the loop again
                loop {
                    iteration_index = iteration_index + 1;
                    if right_lines_information.len() >= iteration_index {
                        break;
                    }
                    let left_content_now_maybe = left_lines_information[iteration_index];
                    let right_content_now_maybe = right_lines_information[iteration_index];
                    if !(left_content_now_maybe.not_present() && right_content_now_maybe.present())
                    {
                        break;
                    } else {
                        let content = get_content_from_file_line_information(
                            &right,
                            &right_content_now_maybe,
                        );
                        match content {
                            Some(content) => {
                                final_lines_file.push(content);
                            }
                            None => {}
                        }
                    }
                }
                break;
            }
            // we have content on both the sides, so we keep going
            if left_content_now_maybe.present() && right_content_now_maybe.present() {
                // things get interesting here, so let's handle each case by case
                let left_color = left_content_now_maybe
                    .get_line_status()
                    .expect("present check above to hold");
                let right_color = right_content_now_maybe
                    .get_line_status()
                    .expect("present check above to hold");
                let left_content =
                    get_content_from_file_line_information(&left, &left_content_now_maybe);
                let right_content =
                    get_content_from_file_line_information(&right, &right_content_now_maybe);
                // no change both are equivalent, best case <3
                if left_color.unchanged() && right_color.unchanged() {
                    let content =
                        get_content_from_file_line_information(&left, &left_content_now_maybe);
                    match content {
                        Some(content) => {
                            final_lines_file.push(content);
                        }
                        None => {}
                    }
                    iteration_index = iteration_index + 1;
                    continue;
                }
                // if we have some color on the left and no color on the right
                // we have to figure out what to do
                // this case represents deletion on the left line and no change
                // on the right line, so we want to keep the left line and not
                // delete it, this is akin to a deletion and insertion
                if left_color.changed() && right_color.unchanged() {
                    // in this case the LLM predicted that we have to remove
                    // a line, this is generally the case with whitespace
                    // otherwise we get a R and G on both sides
                    let content =
                        get_content_from_file_line_information(&left, &left_content_now_maybe);
                    match content {
                        Some(content) => {
                            final_lines_file.push(content);
                        }
                        None => {}
                    };
                    iteration_index = iteration_index + 1;
                    continue;
                }
                if left_color.unchanged() && right_color.changed() {
                    // This is the complicated case we have to handle
                    // this is generally when the LLM wants to edit the file but
                    // whats added here is mostly a comment or something similar
                    // so we can just add the right content here and move on
                    let content =
                        get_content_from_file_line_information(&right, &right_content_now_maybe);
                    match content {
                        Some(content) => {
                            final_lines_file.push(content);
                        }
                        None => {}
                    };
                    iteration_index = iteration_index + 1;
                    continue;
                }
                if left_color.changed() && right_color.changed() {
                    // we do have to insert a diff range here somehow
                    // but how long will be defined by the sequence after this
                    let mut left_content_vec = vec![left_content];
                    let mut right_content_vec = vec![right_content];
                    loop {
                        // the condition we want to look for here is the following
                        // R G
                        // R .
                        // R .
                        // ...
                        // This means that there is a range in the left range
                        // which we have to replace with the Green
                        // we keep going until we have a non-color on the left
                        // or right gets some content
                        iteration_index = iteration_index + 1;
                        if iteration_index >= left_lines_information.len() {
                            // If this happens, we can send a diff with the current
                            // collection
                            final_lines_file.push("<<<<<<<".to_owned());
                            final_lines_file.append(
                                &mut left_content_vec
                                    .into_iter()
                                    .filter_map(|s| s)
                                    .collect::<Vec<String>>(),
                            );
                            final_lines_file.push("=======".to_owned());
                            final_lines_file.append(
                                &mut right_content_vec
                                    .into_iter()
                                    .filter_map(|s| s)
                                    .collect::<Vec<String>>(),
                            );
                            final_lines_file.push(">>>>>>>".to_owned());
                            break;
                        }
                        let left_content_now_maybe = left_lines_information[iteration_index];
                        let right_content_now_maybe = right_lines_information[iteration_index];
                        // if the left content is none here, then we are taking
                        // a L, then we have to break from the loop right now
                        if left_content_now_maybe.not_present() {
                            final_lines_file.push("<<<<<<<".to_owned());
                            final_lines_file.append(
                                &mut left_content_vec.into_iter().filter_map(|s| s).collect(),
                            );
                            final_lines_file.push("=======".to_owned());
                            final_lines_file.append(
                                &mut right_content_vec.into_iter().filter_map(|s| s).collect(),
                            );
                            final_lines_file.push(">>>>>>>".to_owned());
                            break;
                        }
                        let left_color_updated = left_content_now_maybe
                            .get_line_status()
                            .expect("line_status to be present");
                        if left_color_updated == left_color && right_content_now_maybe.not_present()
                        {
                            // we have to keep going here
                            let content = get_content_from_file_line_information(
                                &left,
                                &left_content_now_maybe,
                            );
                            match content {
                                Some(content) => {
                                    left_content_vec.push(Some(content));
                                }
                                None => {}
                            }
                            continue;
                        } else {
                            // we have to break here
                            final_lines_file.push("<<<<<<<".to_owned());
                            final_lines_file.append(
                                &mut left_content_vec.into_iter().flat_map(|s| s).collect(),
                            );
                            final_lines_file.push("=======".to_owned());
                            final_lines_file.append(
                                &mut right_content_vec.into_iter().flat_map(|s| s).collect(),
                            );
                            final_lines_file.push(">>>>>>>".to_owned());
                            break;
                        }
                    }
                    continue;
                }
                break;
            }
        }
    }
    final_lines_file
    // let final_lines_vec = final_lines_file.to_vec();
    // let final_content = final_lines_file.join("\n");
    // println!("=============================================");
    // println!("=============================================");
    // println!("{}", final_content);
    // println!("=============================================");
    // println!("=============================================");
    // final_lines_vec
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DiffActionResponse {
    // Accept the current changes
    AcceptCurrentChanges,
    AcceptIncomingChanges,
    AcceptBothChanges,
}

impl DiffActionResponse {
    pub fn from_gpt_response(response: &str) -> Option<DiffActionResponse> {
        // we are going to parse data between <answer>{your_answer}</answer>
        let response = response
            .split("<answer>")
            .collect::<Vec<_>>()
            .last()
            .unwrap()
            .split("</answer>")
            .collect::<Vec<_>>()
            .first()
            .unwrap()
            .to_owned();
        if response.to_lowercase().contains("accept")
            && response.to_lowercase().contains("current")
            && response.to_lowercase().contains("change")
        {
            return Some(DiffActionResponse::AcceptCurrentChanges);
        }
        if response.to_lowercase().contains("accept")
            && response.to_lowercase().contains("incoming")
            && response.to_lowercase().contains("change")
        {
            return Some(DiffActionResponse::AcceptIncomingChanges);
        }
        if response.to_lowercase().contains("accept")
            && response.to_lowercase().contains("both")
            && response.to_lowercase().contains("change")
        {
            return Some(DiffActionResponse::AcceptBothChanges);
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct FileLineContent {
    pub content: String,
    pub line_content_type: LineContentType,
}

impl FileLineContent {
    pub fn is_diff_start(&self) -> bool {
        matches!(self.line_content_type, LineContentType::DiffStartMarker)
    }

    pub fn is_diff_end(&self) -> bool {
        matches!(self.line_content_type, LineContentType::DiffEndMarker)
    }

    pub fn is_diff_separator(&self) -> bool {
        matches!(self.line_content_type, LineContentType::DiffSeparator)
    }

    pub fn is_line(&self) -> bool {
        matches!(self.line_content_type, LineContentType::FileLine)
    }

    pub fn from_lines(lines: Vec<String>) -> Vec<Self> {
        lines
            .into_iter()
            .map(|content| FileLineContent {
                line_content_type: {
                    if content.contains("<<<<<<<") {
                        LineContentType::DiffStartMarker
                    } else if content.contains("=======") {
                        LineContentType::DiffSeparator
                    } else if content.contains(">>>>>>>") {
                        LineContentType::DiffEndMarker
                    } else {
                        LineContentType::FileLine
                    }
                },
                content,
            })
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone)]
pub enum LineContentType {
    FileLine,
    DiffStartMarker,
    DiffEndMarker,
    DiffSeparator,
}

/// We will use gpt to generate the lines of the code which should be applied
/// to the delta using llm (this is like the machine version of doing git diff(accept/reject))
async fn process_file_lines_to_gpt(
    file_lines: Vec<String>,
    user_query: String,
    session_id: String,
    llm_client: Arc<LlmClient>,
) -> Result<
    Sse<std::pin::Pin<Box<dyn tokio_stream::Stream<Item = anyhow::Result<sse::Event>> + Send>>>,
> {
    // Find where the markers are and then send it over to the llm and ask it
    // to accept/reject the code which has been generated.
    // we detect the git markers and use that for sending over the file and showing that to the LLM
    // we have to check for the <<<<<<, ======, >>>>>> markers and then send the code in between these ranges
    // and 5 lines of prefix to the LLM to ask it to perform the git operation
    // and then use that to build up the file thats how we can solve this
    let edit_messages = async_stream::stream! {
        let mut initial_index = 0;
        let file_lines_to_document = FileLineContent::from_lines(file_lines);
        let total_lines = file_lines_to_document.len();
        let mut total_file_lines: Vec<String> = vec![];
        let mut edit_responses = vec![];
        while initial_index < total_lines {
            let line = file_lines_to_document[initial_index].clone();
            if line.is_diff_start() {
                let mut current_changes = vec![];
                let mut current_iteration_index = initial_index + 1;
                while !file_lines_to_document[current_iteration_index].is_diff_separator() {
                    // we have to keep going here
                    current_changes.push(
                        file_lines_to_document[current_iteration_index]
                            .content
                            .to_owned(),
                    );
                    current_iteration_index = current_iteration_index + 1;
                }
                // Now we are at the index which has ======, so move to the next one
                current_iteration_index = current_iteration_index + 1;
                let mut incoming_changes = vec![];
                while !file_lines_to_document[current_iteration_index].is_diff_end() {
                    // we have to keep going here
                    incoming_changes.push(
                        file_lines_to_document[current_iteration_index]
                            .content
                            .to_owned(),
                    );
                    current_iteration_index = current_iteration_index + 1;
                }
                // This is where we will call the agent out and ask it to decide
                // which of the following git diffs to keep and which to remove
                // before we do this, we can do some hand-woven checks to figure out
                // what action to take
                // we also want to keep a prefix of the lines here and send that along
                // to the llm for context as well
                let (delta_lines, edit_file_response) = call_gpt_for_action_resolution(
                    // the index is simply the length of the current lines which are
                    // present in the file
                    total_file_lines.len(),
                    current_changes,
                    incoming_changes,
                    total_file_lines
                        .iter()
                        .rev()
                        .take(5)
                        .rev()
                        .into_iter()
                        .map(|s| s.to_owned())
                        .collect::<Vec<_>>(),
                    &user_query,
                    llm_client.clone(),
                )
                .await;
                total_file_lines.extend(delta_lines.to_vec());
                println!("===== selection lines =====");
                println!("{}", delta_lines.to_vec().join("\n"));
                println!("===== selection lines =====");
                println!("==============================");
                println!("==============================");
                println!("{}", total_file_lines.join("\n"));
                println!("==============================");
                println!("==============================");
                if let Some(edit_response) = edit_file_response {
                    edit_responses.push(edit_response.clone());
                    yield edit_response;
                }
                // Now we are at the index which has >>>>>>>, so move to the next one on the iteration loop
                initial_index = current_iteration_index + 1;
                // we have a git diff event now, so lets try to fix that
            } else {
                // just insert the line here and then push the current line to the
                // total_file_lines
                // we know here that it will be a normal line, so we can just add
                // the content back
                total_file_lines.push(line.content);
                initial_index = initial_index + 1;
            }
        }
    };
    let cloned_session_id = session_id.to_owned();
    let init_stream = futures::stream::once(async move {
        Ok(sse::Event::default()
            .json_data(EditFileResponse::Status {
                session_id: session_id.clone(),
                status: "started".to_owned(),
            })
            // This should never happen, so we force an unwrap.
            .expect("failed to serialize initialization object"))
    });
    let answer_stream = edit_messages.map(|item| {
        Ok(sse::Event::default()
            .json_data(item)
            .expect("failed to serialize edit response"))
    });
    let done_stream = futures::stream::once(async move {
        Ok(sse::Event::default()
            .json_data(EditFileResponse::Status {
                session_id: cloned_session_id,
                status: "done".to_owned(),
            })
            .expect("failed to send done object"))
    });
    let stream = init_stream.chain(answer_stream).chain(done_stream);
    // Fix this type error which is happening here
    Ok(Sse::new(Box::pin(stream)))
}

async fn call_gpt_for_action_resolution(
    // where do the line changes start from
    line_start: usize,
    current_changes: Vec<String>,
    incoming_changes: Vec<String>,
    prefix: Vec<String>,
    query: &str,
    llm_client: Arc<LlmClient>,
) -> (Vec<String>, Option<EditFileResponse>) {
    let system_message = llm_funcs::llm::Message::system(&diff_accept_prompt(query));
    let user_messages = prompts::diff_user_messages(
        &prefix.join("\n"),
        &current_changes.join("\n"),
        &incoming_changes.join("\n"),
    )
    .into_iter()
    .map(|message| llm_funcs::llm::Message::user(&message));
    let messages = vec![system_message]
        .into_iter()
        .chain(user_messages)
        .collect::<Vec<_>>();
    let model = llm_funcs::llm::OpenAIModel::GPT4;
    let response = llm_client.response(model, messages, None, 0.1, None).await;
    dbg!(&response);
    let diff_action = match response {
        Ok(response) => DiffActionResponse::from_gpt_response(&response),
        Err(_) => {
            // leave it as it is
            None
        }
    };
    match diff_action {
        Some(DiffActionResponse::AcceptCurrentChanges) => {
            // we have to accept the current changes
            (current_changes, None)
        }
        Some(DiffActionResponse::AcceptIncomingChanges) => {
            // we have to accept the incoming changes
            let content = incoming_changes.join("\n");
            (
                incoming_changes,
                Some(EditFileResponse::TextEdit {
                    range: Range::new(
                        Position::new(line_start, 0, 0),
                        // large number here for the column end value for the end line
                        Position::new(line_start + current_changes.len(), 10000, 0),
                    ),
                    content,
                }),
            )
        }
        Some(DiffActionResponse::AcceptBothChanges) => {
            // we have to accept both the changes
            let current_change_size = current_changes.len();
            let changes = current_changes
                .into_iter()
                .chain(incoming_changes)
                .collect::<Vec<_>>();
            let changes_str = changes.join("\n");
            (
                changes,
                Some(EditFileResponse::TextEdit {
                    range: Range::new(
                        Position::new(line_start, 0, 0),
                        Position::new(line_start + current_change_size, 1000, 0),
                    ),
                    content: changes_str,
                }),
            )
        }
        None => {
            // we have to accept the current changes
            (current_changes, None)
        }
    }
}
