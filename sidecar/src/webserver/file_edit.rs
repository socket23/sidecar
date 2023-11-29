use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::response::{sse, IntoResponse, Sse};
use axum::{Extension, Json};
use difftastic::LineInformation;
use either::Either;
use futures::{stream, FutureExt, StreamExt};
use regex::Regex;

use crate::agent::llm_funcs::LlmClient;
use crate::agent::prompts::diff_accept_prompt;
use crate::agent::types::Answer;
use crate::agent::{llm_funcs, prompts};
use crate::application::application::Application;
use crate::chunking::languages::TSLanguageParsing;
use crate::chunking::text_document::{Position, Range};
use crate::chunking::types::{
    ClassInformation, ClassNodeType, FunctionInformation, TypeInformation,
};
use crate::in_line_agent::context_parsing::{generate_selection_context, ContextWindowTracker};
use crate::in_line_agent::types::ContextSelection;

use super::types::{ApiResponse, Result};

const TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditFileRequest {
    pub file_path: String,
    pub file_content: String,
    pub new_content: String,
    pub language: String,
    pub user_query: String,
    pub session_id: String,
    pub code_block_index: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TextEditStreaming {
    Start {
        code_block_index: usize,
        context_selection: ContextSelection,
    },
    EditStreaming {
        code_block_index: usize,
        range: Range,
        content_up_until_now: String,
        content_delta: String,
    },
    End {
        code_block_index: usize,
        reason: String,
    },
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
        should_insert: bool,
    },
    TextEditStreaming {
        data: TextEditStreaming,
    },
    Status {
        session_id: String,
        status: String,
    },
}

impl EditFileResponse {
    fn start_text_edit(context_selection: ContextSelection, code_block_index: usize) -> Self {
        Self::TextEditStreaming {
            data: TextEditStreaming::Start {
                context_selection,
                code_block_index,
            },
        }
    }

    fn end_text_edit(code_block_index: usize) -> Self {
        Self::TextEditStreaming {
            data: TextEditStreaming::End {
                reason: "done".to_owned(),
                code_block_index,
            },
        }
    }

    fn stream_edit(range: Range, content: String, code_block_index: usize) -> Self {
        Self::TextEditStreaming {
            data: TextEditStreaming::EditStreaming {
                range,
                content_up_until_now: content.to_owned(),
                content_delta: content,
                code_block_index,
            },
        }
    }

    fn stream_incremental_edit(
        range: &Range,
        buf: String,
        delta: String,
        code_block_index: usize,
    ) -> Self {
        Self::TextEditStreaming {
            data: TextEditStreaming::EditStreaming {
                range: range.clone(),
                content_up_until_now: buf,
                content_delta: delta,
                code_block_index,
            },
        }
    }
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
        code_block_index,
    }): Json<EditFileRequest>,
) -> Result<impl IntoResponse> {
    // Here we have to first check if the new content is tree-sitter valid, if
    // thats the case only then can we apply it to the file
    // First we check if the output generated is valid by itself, if it is then
    // we can think about applying the changes to the file
    let llm_client = Arc::new(LlmClient::codestory_infra(
        app.posthog_client.clone(),
        app.sql.clone(),
        app.user_id.to_owned(),
        app.llm_config.clone(),
    ));
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
        let nearest_range_for_symbols = find_nearest_position_for_code_edit(
            &file_content,
            &file_path,
            &new_content,
            &language,
            app.language_parsing.clone(),
        )
        .await;

        // Now we apply the edits and send it over to the user
        // After generating the git diff we want to send back the responses to the
        // user depending on what edit information we get, we can stream this to the
        // user so they know the agent is working on some action and it will show up
        // as edits on the editor
        let split_lines = Regex::new(r"\r\n|\r|\n").unwrap();
        let file_lines: Vec<String> = split_lines
            .split(&file_content)
            .map(|s| s.to_owned())
            .collect();

        let result = llm_writing_code(
            file_lines,
            file_content,
            new_content,
            user_query,
            language,
            session_id,
            llm_client,
            app.language_parsing.clone(),
            file_path,
            nearest_range_for_symbols,
            code_block_index,
        )
        .await;
        result
    }
}

// We use this enum as a placeholder for the different type of variables which we support exporting at the
// moment
#[derive(Debug, Clone)]
enum CodeSymbolInformation {
    Class(ClassInformation),
    Function(FunctionInformation),
    Type(TypeInformation),
}

impl CodeSymbolInformation {
    pub fn content(&self, file_content: &str) -> String {
        match self {
            CodeSymbolInformation::Class(class_information) => {
                class_information.content(file_content)
            }
            CodeSymbolInformation::Function(function_information) => {
                function_information.content(file_content)
            }
            CodeSymbolInformation::Type(type_information) => type_information.content(file_content),
        }
    }

    pub fn name(&self) -> String {
        match self {
            CodeSymbolInformation::Class(class_information) => {
                class_information.get_name().to_owned()
            }
            CodeSymbolInformation::Function(function_information) => function_information
                .name()
                .map(|name| name.to_owned())
                .unwrap_or_default(),
            CodeSymbolInformation::Type(type_information) => type_information.name.to_owned(),
        }
    }

    pub fn symbol_type(&self) -> String {
        match self {
            CodeSymbolInformation::Class(_) => "class".to_owned(),
            CodeSymbolInformation::Function(_) => "function".to_owned(),
            CodeSymbolInformation::Type(_) => "type".to_owned(),
        }
    }

    fn merge_symbols_from_index(
        symbols_vec: Vec<CodeSymbolInformation>,
        start_index: usize,
        file_content: &str,
    ) -> String {
        let mut symbols_vec = symbols_vec;
        let mut final_string = "".to_owned();

        for symbol in symbols_vec.drain(start_index..) {
            final_string.push_str(&symbol.content(file_content));
            final_string.push('\n');
        }
        final_string
    }
}

async fn find_nearest_position_for_code_edit(
    file_content: &str,
    file_path: &str,
    new_content: &str,
    language: &str,
    language_parsing: Arc<TSLanguageParsing>,
) -> Vec<(Option<Range>, CodeSymbolInformation)> {
    // Steps taken:
    // - First get all the classes and functions which are present in the code blocks provided
    // - Get the types which are provided in the code block as well (these might be types or anything else in typescript)
    // - Search the current open file to see if this already exists in the file
    // - If it exists we have a more restricted area to apply the diff to
    // - Handle the imports properly as always
    let language_parser = language_parsing.for_lang(language);
    if language_parser.is_none() {
        return vec![];
    }
    let language_parser = language_parser.unwrap();
    if !language_parser.is_valid_code(new_content) {
        return vec![];
    }
    let class_with_funcs_llm = language_parser.generate_file_symbols(new_content.as_bytes());
    let class_with_funcs = language_parser.generate_file_symbols(file_content.as_bytes());
    let types_llm = language_parser.capture_type_data(new_content.as_bytes());
    let types_file = language_parser.capture_type_data(file_content.as_bytes());
    // First we want to try and match all the classes as much as possible
    // then we will look at the individual functions and try to match them

    // These are the functions which are prensent in the class of the file
    let class_functions_from_file = class_with_funcs_llm
        .to_vec()
        .into_iter()
        .filter_map(|class_with_func| {
            if class_with_func.class_information.is_some() {
                Some(class_with_func.function_information)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();
    // These are the classes which the llm has generated (we use it to only match with other classes)
    let classes_llm_generated = class_with_funcs_llm
        .to_vec()
        .into_iter()
        .filter_map(|class_with_func| {
            if class_with_func.class_information.is_some() {
                Some(class_with_func.class_information)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();
    // These are the classes which are present in the file
    let classes_from_file = class_with_funcs
        .to_vec()
        .into_iter()
        .filter_map(|class_with_func| {
            if class_with_func.class_information.is_some() {
                Some(class_with_func.class_information)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();
    // These are the independent functions which the llm has generated
    let independent_functions_llm_generated = class_with_funcs_llm
        .into_iter()
        .filter_map(|class_with_func| {
            if class_with_func.class_information.is_none() {
                Some(class_with_func.function_information)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();
    // These are the independent functions which are present in the file
    let independent_functions_from_file = class_with_funcs
        .into_iter()
        .filter_map(|class_with_func| {
            if class_with_func.class_information.is_none() {
                Some(class_with_func.function_information)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    // Now we try to check if any of the functions match,
    // if they do we capture the matching range in the original value, this allows us to have a finer area to apply the diff to
    let llm_functions_to_range = independent_functions_llm_generated
        .into_iter()
        .map(|function_llm| {
            let node_information = function_llm.get_node_information();
            match node_information {
                Some(node_information) => {
                    let function_name_llm = node_information.get_name();
                    let parameters_llm = node_information.get_parameters();
                    let return_type_llm = node_information.get_return_type();
                    // We have the 3 identifiers above to figure out which function can match with this, if none match then we know
                    // that this is a new function and we should treat it as such
                    let mut found_function_vec = independent_functions_from_file
                        .iter()
                        .filter_map(|function_information| {
                            let node_information = function_information.get_node_information();
                            match node_information {
                                Some(node_information) => {
                                    let function_name = node_information.get_name();
                                    let parameters = node_information.get_parameters();
                                    let return_type = node_information.get_return_type();
                                    let score = (function_name_llm == function_name) as usize
                                        + (parameters_llm == parameters) as usize
                                        + (return_type_llm == return_type) as usize;
                                    // We have the 3 identifiers above to figure out which function can match with this, if none match then we know
                                    // that this is a new function and we should treat it as such
                                    if score == 0 || function_name_llm != function_name {
                                        None
                                    } else {
                                        Some((score, function_information.clone()))
                                    }
                                }
                                None => None,
                            }
                        })
                        .collect::<Vec<_>>();
                    found_function_vec.sort_by(|a, b| b.0.cmp(&a.0));
                    let found_function = found_function_vec
                        .first()
                        .map(|(_, function_information)| function_information);
                    if let Some(found_function) = found_function {
                        // We have a match! let's lock onto the range of this function node which we found and then
                        // we can go about applying the diff to this range
                        return (Some(found_function.range().clone()), function_llm);
                    }

                    // Now it might happen that these functions are part of the clas function, in which case
                    // we should check the class functions as well to figure out if that's the case and we can
                    // get the correct range that way
                    let found_function =
                        class_functions_from_file
                            .iter()
                            .find(|function_information| {
                                let node_information = function_information.get_node_information();
                                match node_information {
                                    Some(node_information) => {
                                        let function_name = node_information.get_name();
                                        let parameters = node_information.get_parameters();
                                        let return_type = node_information.get_return_type();
                                        let score = (function_name_llm == function_name) as usize
                                            + (parameters_llm == parameters) as usize
                                            + (return_type_llm == return_type) as usize;
                                        // We have the 3 identifiers above to figure out which function can match with this, if none match then we know
                                        // that this is a new function and we should treat it as such
                                        if score == 0 || function_name_llm != function_name {
                                            false
                                        } else {
                                            true
                                        }
                                    }
                                    None => false,
                                }
                            });
                    if let Some(found_function) = found_function {
                        // We have a match! let's lock onto the range of this function node which we found and then
                        // we can go about applying the diff to this range
                        return (Some(found_function.range().clone()), function_llm);
                    }
                    // If the class function finding also fails, then we just return None here :(
                    // since it might be a new function at this point?
                    (None, function_llm)
                }
                None => (None, function_llm),
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(range, function)| (range, CodeSymbolInformation::Function(function)))
        .collect::<Vec<_>>();

    // Now we have to try and match the classes in the same way, so we can figure out if we have a smaller range to apply the diff
    let llm_classes_to_range = classes_llm_generated
        .into_iter()
        .map(|llm_class_information| {
            let class_identifier = llm_class_information.get_name();
            let class_type = llm_class_information.get_class_type();
            match class_type {
                ClassNodeType::ClassDeclaration => {
                    // Try to find which class in the original file this could match with
                    let possible_class = classes_from_file
                        .iter()
                        .find(|class_information| class_information.get_name() == class_identifier);
                    match possible_class {
                        // yay, happy path we found some class, lets return this as the range for the class right now
                        Some(possible_class) => {
                            (Some(possible_class.range().clone()), llm_class_information)
                        }
                        None => (None, llm_class_information),
                    }
                }
                ClassNodeType::Identifier => (None, llm_class_information),
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(range, class)| (range, CodeSymbolInformation::Class(class)))
        .collect::<Vec<_>>();

    // Now we try to get the types which the llm has suggested and which might be also present in the file
    // this allows us to figure out the delta between them
    let llm_types_to_range = types_llm
        .into_iter()
        .map(|llm_type_information| {
            let type_identifier = llm_type_information.name.to_owned();
            let possible_type = types_file
                .iter()
                .find(|type_information| type_information.name == type_identifier);
            match possible_type {
                // yay, happy path we found some type, lets return this as the range for the type right now
                Some(possible_type) => (Some(possible_type.range.clone()), llm_type_information),
                None => (None, llm_type_information),
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(range, type_information)| (range, CodeSymbolInformation::Type(type_information)))
        .collect::<Vec<_>>();

    // TODO(skcd): Now we have classes and functions which are mapped to their actual representations in the file
    // this is very useful since our diff application can be more coherent now and we can send over more
    // correct data, but what about the things that we missed? let's get to them in a bit, focus on these first

    // First we have to order the functions and classes in the order of their ranges
    let mut identified: Vec<(Option<Range>, CodeSymbolInformation)> = llm_functions_to_range
        .into_iter()
        .chain(llm_classes_to_range)
        .chain(llm_types_to_range)
        .collect();
    identified.sort_by(|a, b| match (a.0.as_ref(), b.0.as_ref()) {
        (Some(a_range), Some(b_range)) => a_range.start_byte().cmp(&b_range.start_byte()),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    identified
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
    // let validity = language_parser.is_valid_code(file_content);
    // dbg!("language", language);
    // if !validity {
    //     dbg!("file_content is not valid for language parsing");
    //     return None;
    // }
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
                // we need to send the git markers for this anyways, since its important
                // for the editor to know that some insertion has happened
                final_lines_file.push("<<<<<<<".to_owned());
                final_lines_file.push("=======".to_owned());
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
                        final_lines_file.push(">>>>>>>".to_owned());
                        break;
                    }
                    let left_content_now_maybe = left_lines_information[iteration_index];
                    let right_content_now_maybe = right_lines_information[iteration_index];
                    if !(left_content_now_maybe.not_present() && right_content_now_maybe.present())
                    {
                        // we are not in the same style as before, so we break it
                        final_lines_file.push(">>>>>>>".to_owned());
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
                    let right_content_vec = vec![right_content];
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
    pub fn get_content(&self) -> String {
        self.content.to_owned()
    }

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

async fn llm_writing_code(
    file_lines: Vec<String>,
    file_content: String,
    llm_content: String,
    user_query: String,
    language: String,
    session_id: String,
    llm_client: Arc<LlmClient>,
    language_parsing: Arc<TSLanguageParsing>,
    file_path: String,
    nearest_range_symbols: Vec<(Option<Range>, CodeSymbolInformation)>,
    code_block_index: usize,
) -> Result<
    Sse<std::pin::Pin<Box<dyn tokio_stream::Stream<Item = anyhow::Result<sse::Event>> + Send>>>,
> {
    // Here we have to generate the code using the llm and then we have to apply the diff
    let edit_messages = async_stream::stream! {
        let mut initial_index = 0;
        let total_lines = file_lines.len();
        let mut total_file_lines: Vec<String> = vec![];
        let mut nearest_range_symbols_index = 0;
        let nearest_range_symbols_len = nearest_range_symbols.len();
        let cloned_language_parsing = language_parsing.clone();
        while initial_index <= total_lines {
            // now we have the symbols and the range we want to replace
            if nearest_range_symbols_index >= nearest_range_symbols_len {
                break;
            }
            let (file_symbol_range_maybe, _) = nearest_range_symbols[nearest_range_symbols_index].clone();
            if let None = file_symbol_range_maybe {
                // At this point, we don't have a range, and the rest of the symbols can be concatenated and sent over
                // the wire
               let merged_symbols = CodeSymbolInformation::merge_symbols_from_index(
                    nearest_range_symbols
                        .to_vec()
                        .into_iter()
                        .map(|(_, class_or_function)| class_or_function)
                        .collect::<Vec<_>>(),
                    nearest_range_symbols_index,
                    &llm_content,
                );
                let formatted_merged_symbols = format!(r#"```{language}
// FILEPATH: new_content.ts
// BEGIN: LLM
{merged_symbols}
// END: LLM
```"#);
                // add the rest of the lines to the lines of the file as well
                total_file_lines.append(
                    file_lines
                        .clone()
                        .into_iter()
                        .skip(initial_index)
                        .collect::<Vec<_>>()
                        .as_mut(),
                );
                // We have to also send the selection context here, since the editor uses
                // this for figuring out the tabs and spaces for the generated content
                let start_line = if initial_index == 0 { 0 } else { initial_index - 1 };
                let replacement_range = Range::new(
                    Position::new(start_line, 100_000, 100_000),
                    Position::new(total_file_lines.len() - 1, 100_000, 100_000),
                );
                let total_lines_now = total_file_lines.len();
                let selection_context = ContextSelection::generate_placeholder_for_range(
                    &replacement_range,
                );
                // now we can send over the events to the client
                yield EditFileResponse::start_text_edit(selection_context, code_block_index);
                // sending the request
                yield EditFileResponse::stream_edit(Range::new(
                    Position::new(total_lines_now - 1, 100_000, 100_000),
                    Position::new(total_lines_now - 1, 100_000, 100_000),
                ), formatted_merged_symbols, code_block_index);
                yield EditFileResponse::end_text_edit(code_block_index);
                // we break from the loop here, since we have nothing else to do after this
                break;
            } else {
                // If we are at the limits of the line, then we should break because all replacements here
                // should happen within the file
                if initial_index >= total_lines {
                    break;
                }
                // we need to start the range here
                let file_symbol_range = file_symbol_range_maybe.expect("if let None holds true");
                // This is the content of the code symbol from the file
                let file_code_symbol = file_content[file_symbol_range.start_byte()..file_symbol_range.end_byte()].to_owned();

                let start_line = file_symbol_range.start_position().line();
                if initial_index < start_line {
                    total_file_lines.push(file_lines[initial_index].to_owned());
                    initial_index = initial_index + 1;
                    continue;
                }
                if initial_index == start_line {
                    let symbol_name = nearest_range_symbols[nearest_range_symbols_index]
                        .1
                        .name();
                    let symbol_type = nearest_range_symbols[nearest_range_symbols_index]
                        .1
                        .symbol_type();
                    let llm_symbol_content = nearest_range_symbols[nearest_range_symbols_index]
                        .1
                        .content(&llm_content);
                    let git_diff = generate_file_diff(
                        &file_code_symbol,
                        &file_path,
                        &llm_symbol_content,
                        &language,
                        cloned_language_parsing.clone(),
                    ).await
                    .map(|content| {
                        if content.is_empty() {
                            llm_symbol_content.to_owned()
                        } else {
                            content.join("\n")
                        }
                    }).unwrap_or(llm_symbol_content.to_owned());

                    // we have a match with the start of a symbol, so lets try to ask gpt to stream it
                    let messages = vec![
                        llm_funcs::llm::Message::system(&prompts::system_prompt_for_git_patch(&user_query, &language, &symbol_name, &symbol_type))
                    ].into_iter().chain(
                        prompts::user_message_for_git_patch(
                            &language,
                            &symbol_name,
                            &symbol_type,
                            &git_diff,
                            &file_path,
                            &file_code_symbol,
                        )
                    .into_iter().map(|s| llm_funcs::llm::Message::user(&s)))
                    .collect::<Vec<_>>();
                    // Now we send it over to gpt3.5 and ask it to generate code
                    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Answer>();
                    let reciever_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver).map(|item| either::Left(item));
                    let llm_answer = llm_client.stream_response(llm_funcs::llm::OpenAIModel::GPT3_5_16k, messages, None, 0.1, None, sender).into_stream().map(|item| either::Right(item));
                    // we drain the answer stream here and send over our incremental edits update
                    let timeout = Duration::from_secs(TIMEOUT_SECS);

                    // over here as well, we need to generate the context selection and send it over
                    // to the editor for spaces and tabs and everything else


                    // First send the start of text edit
                    // Since we will be rewriting the whole symbol what we know about the ranges
                    // here are as follows:
                    // - we have the prefix before this stored in our total_file_lines
                    // - we have the suffix of the file stored after the symbol range
                    // - we can combine and send that over
                    // - in between we can put placeholder because it does not matter

                    let mut suffix = vec![];
                    file_lines.iter().skip(file_symbol_range.end_line() + 1).take(20).for_each(|line| suffix.push(line.to_owned()));
                    let total_lines = total_file_lines.len() + (file_symbol_range.end_line() - file_symbol_range.start_line() + 1) + suffix.len();
                    let empty_lines = vec!["".to_owned(); file_symbol_range.end_line() - file_symbol_range.start_line() + 1];
                    let selection_context = generate_selection_context(
                        total_lines as i64,
                        &file_symbol_range,
                        &file_symbol_range,
                        &Range::new(Position::new(0, 0, 0), Position::new(total_lines - 1, 1000, 1000)),
                        &language,
                        total_file_lines.to_vec().into_iter().chain(empty_lines.into_iter()).chain(suffix.into_iter()).collect::<Vec<_>>(),
                        file_path.to_owned(),
                        &mut ContextWindowTracker::large_window(),
                    ).to_context_selection();
                    yield EditFileResponse::start_text_edit(selection_context, code_block_index);
                    for await item in tokio_stream::StreamExt::timeout(
                        stream::select(reciever_stream, llm_answer),
                        timeout,
                    ) {
                        match item {
                            Ok(Either::Left(answer_item)) => {
                                if let Some(delta) = answer_item.delta {
                                    // stream the incremental edits here
                                    yield EditFileResponse::stream_incremental_edit(&file_symbol_range, answer_item.answer_up_until_now, delta, code_block_index);
                                }
                            },
                            Ok(Either::Right(Ok(llm_answer))) => {
                                // Now that we have the final answer, we can update our total lines and move the index to after
                                // always end the text edit
                                yield EditFileResponse::end_text_edit(code_block_index);
                                let code_snippet = llm_answer.split("// BEGIN: be15d9bcejpp\n").collect::<Vec<_>>().last().unwrap().split("\n// END: be15d9bcejpp\n").collect::<Vec<_>>().first().unwrap().to_owned();
                                total_file_lines.append(
                                    code_snippet.split("\n").collect::<Vec<_>>().into_iter().map(|s| s.to_owned()).collect::<Vec<_>>().as_mut(),
                                );
                                initial_index = file_symbol_range.end_position().line() + 1;
                                nearest_range_symbols_index = nearest_range_symbols_index + 1;
                                break;
                            },
                            _ => {
                                // If things fail, then we skip the current symbol which we want to change
                                // and we want to move to modifying the next range
                                initial_index = file_symbol_range.end_position().line() + 1;
                                nearest_range_symbols_index = nearest_range_symbols_index + 1;
                                // end the text edit always
                                yield EditFileResponse::end_text_edit(code_block_index);
                            },
                        }
                    }
                }
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
    Ok(Sse::new(Box::pin(stream)))
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
            // we want to go until we have a diff start marker
            if line.is_diff_start() {
                let mut current_changes = vec![];
                let mut current_iteration_index = initial_index + 1;
                // Here we are iterating until we hit the diff separator
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
                // Here we are iterating until we hit the diff end
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
                let prefix = total_file_lines.iter().rev().take(5).rev().into_iter().map(|s| s.to_owned()).collect::<Vec<_>>();
                // Now we want to provide the suffix as well, this is interesting because
                // we can have unresolved merge conflicts at this point so the llm will need
                // to be prompted accordingly or else it will get confused
                let mut suffix_index = current_iteration_index + 1; // +1 here since current_iteration_index is the index of the diff end marker
                let iteratation_limit = 5;
                let mut suffix_lines = vec![];
                while suffix_index < total_lines && suffix_lines.len() < iteratation_limit {
                    let line = file_lines_to_document[suffix_index].get_content();
                    suffix_index = suffix_index + 1;
                    suffix_lines.push(line);
                }

                // we want to get some suffix here so we can pass that along to the LLM
                let (delta_lines, edit_file_response) = call_gpt_for_action_resolution(
                    // the index is simply the length of the current lines which are
                    // present in the file
                    total_file_lines.len(),
                    current_changes,
                    incoming_changes,
                    prefix,
                    suffix_lines,
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
    Ok(Sse::new(Box::pin(stream)))
}

async fn call_gpt_for_action_resolution(
    // where do the line changes start from
    line_start: usize,
    current_changes: Vec<String>,
    incoming_changes: Vec<String>,
    prefix: Vec<String>,
    suffix: Vec<String>,
    query: &str,
    llm_client: Arc<LlmClient>,
) -> (Vec<String>, Option<EditFileResponse>) {
    // we can handle some edge cases early on and save a llm call
    // case 1: if we have nothing on the left side, but have to insert on the right side we already
    // know the verdict for it
    if current_changes.is_empty() && !incoming_changes.is_empty() {
        let mut content = incoming_changes.join("\n");
        content = content + "\n";
        return (
            incoming_changes,
            Some(EditFileResponse::TextEdit {
                range: Range::new(
                    Position::new(line_start, 0, 0),
                    // large number here for the column end value for the end line
                    Position::new(line_start + current_changes.len(), 10000, 0),
                ),
                content,
                should_insert: true,
            }),
        );
    }
    let system_message = llm_funcs::llm::Message::system(&diff_accept_prompt(query));
    let user_messages = prompts::diff_user_messages(
        &prefix.join("\n"),
        &current_changes.join("\n"),
        &incoming_changes.join("\n"),
        &suffix.join("\n"),
    )
    .into_iter()
    .map(|message| llm_funcs::llm::Message::user(&message));
    let messages = vec![system_message]
        .into_iter()
        .chain(user_messages)
        .collect::<Vec<_>>();
    let model = llm_funcs::llm::OpenAIModel::GPT4;
    let response = llm_client.response(model, messages, None, 0.1, None).await;
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
            let mut content = incoming_changes.join("\n");
            content = content + "\n";
            (
                incoming_changes,
                Some(EditFileResponse::TextEdit {
                    range: Range::new(
                        Position::new(line_start, 0, 0),
                        // large number here for the column end value for the end line
                        Position::new(line_start + current_changes.len(), 10000, 0),
                    ),
                    content,
                    should_insert: false,
                }),
            )
        }
        Some(DiffActionResponse::AcceptBothChanges) => {
            // we have to accept both the changes, since the vscode api only allows for
            // edit or insert, here we will insert the other part of our changes after moving
            // the line index to where the current changes will be
            let current_change_size = current_changes.len();
            let incoming_changes_start_index = line_start + current_change_size;
            let incoming_changes_end_index: usize =
                incoming_changes_start_index + incoming_changes.len();
            let mut changes_str = incoming_changes.join("\n");
            // send an extra \n here because we are inserting the changes and when we join,
            // we are going to miss the last \n so we want to move the pointer to the next line
            changes_str = changes_str + "\n";

            let changes = current_changes
                .into_iter()
                .chain(incoming_changes)
                .collect::<Vec<_>>();

            (
                changes,
                Some(EditFileResponse::TextEdit {
                    range: Range::new(
                        Position::new(incoming_changes_start_index, 0, 0),
                        Position::new(incoming_changes_end_index, 1000, 0),
                    ),
                    content: changes_str,
                    should_insert: true,
                }),
            )
        }
        None => {
            // we have to accept the current changes
            (current_changes, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::chunking::languages::TSLanguageParsing;

    use super::generate_file_diff;

    #[tokio::test]
    async fn test_generate_git_diff_output_typescript_function_with_comments() {
        let file_content = r#"
function getContextFromEditor(editor: ICodeEditor, accessor: ServicesAccessor): IChatCodeBlockActionContext | undefined {
    const chatWidgetService = accessor.get(ICSChatWidgetService);
    const model = editor.getModel();
    if (!model) {
        return;
    }

    const widget = chatWidgetService.lastFocusedWidget;
    if (!widget) {
        return;
    }

    const codeBlockInfo = widget.getCodeBlockInfoForEditor(model.uri);
    if (!codeBlockInfo) {
        return;
    }

    return {
        element: codeBlockInfo.element,
        codeBlockIndex: codeBlockInfo.codeBlockIndex,
        code: editor.getValue(),
        languageId: editor.getModel()!.getLanguageId(),
    };
}"#;
        let file_path = "testing.ts";
        let new_content = r#"
function getContextFromEditor(editor: ICodeEditor, accessor: ServicesAccessor): IChatCodeBlockActionContext | undefined {
    // Get the chat widget service from the accessor
    const chatWidgetService = accessor.get(ICSChatWidgetService);
    // Get the model from the editor
    const model = editor.getModel();
    // If there is no model, return undefined
    if (!model) {
        return;
    }

    // Get the last focused widget from the chat widget service
    const widget = chatWidgetService.lastFocusedWidget;
    // If there is no widget, return undefined
    if (!widget) {
        return;
    }

    // Get the code block info for the editor from the widget
    const codeBlockInfo = widget.getCodeBlockInfoForEditor(model.uri);
    // If there is no code block info, return undefined
    if (!codeBlockInfo) {
        return;
    }

    // Return an object containing the element, code block index, code, and language ID
    return {
        element: codeBlockInfo.element,
        codeBlockIndex: codeBlockInfo.codeBlockIndex,
        code: editor.getValue(),
        languageId: editor.getModel()!.getLanguageId(),
    };
}"#;
        let language = "typescript";
        let language_parsing = Arc::new(TSLanguageParsing::init());
        let file_diff = generate_file_diff(
            file_content,
            file_path,
            new_content,
            language,
            language_parsing,
        )
        .await;
        assert!(file_diff.is_some());
        let git_diff = file_diff.expect("to be present").join("\n");
        //     let expected_git_diff = r#"
        // function getContextFromEditor(editor: ICodeEditor, accessor: ServicesAccessor): IChatCodeBlockActionContext | undefined {
        // <<<<<<<
        // =======
        //     // Get the chat widget service from the accessor
        // >>>>>>>
        //     const chatWidgetService = accessor.get(ICSChatWidgetService);
        // <<<<<<<
        // =======
        //     // Get the model from the editor
        // >>>>>>>
        //     const model = editor.getModel();
        // <<<<<<<
        // =======
        //     // If there is no model, return undefined
        // >>>>>>>
        //     if (!model) {
        //         return;
        //     }

        // <<<<<<<
        // =======
        //     // Get the last focused widget from the chat widget service
        // >>>>>>>
        //     const widget = chatWidgetService.lastFocusedWidget;
        // <<<<<<<
        // =======
        //     // If there is no widget, return undefined
        // >>>>>>>
        //     if (!widget) {
        //         return;
        //     }

        // <<<<<<<
        // =======
        //     // Get the code block info for the editor from the widget
        // >>>>>>>
        //     const codeBlockInfo = widget.getCodeBlockInfoForEditor(model.uri);
        // <<<<<<<
        // =======
        //     // If there is no code block info, return undefined
        // >>>>>>>
        //     if (!codeBlockInfo) {
        //         return;
        //     }

        // <<<<<<<
        // =======
        //     // Return an object containing the element, code block index, code, and language ID
        // >>>>>>>
        //     return {
        //         element: codeBlockInfo.element,
        //         codeBlockIndex: codeBlockInfo.codeBlockIndex,
        //         code: editor.getValue(),
        //         languageId: editor.getModel()!.getLanguageId(),
        //     };
        // }"#;
        //     assert_eq!(git_diff, expected_git_diff);
    }
}
