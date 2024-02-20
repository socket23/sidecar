use std::pin::Pin;
use std::sync::Arc;

use chrono::Local;
use futures::{stream, StreamExt};
use futures::{FutureExt, Stream};
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionStringRequest, LLMType},
    tokenizer::tokenizer::{LLMTokenizer, LLMTokenizerError},
};
use llm_prompts::{
    answer_model::LLMAnswerModelBroker,
    fim::types::{FillInMiddleBroker, FillInMiddleRequest},
};

use crate::inline_completion::context::clipboard_context::{
    ClipboardContext, ClipboardContextString,
};
use crate::{
    chunking::editor_parsing::EditorParsing,
    webserver::inline_completion::{
        InlineCompletion, InlineCompletionRequest, InlineCompletionResponse,
    },
};

use super::{
    context::{current_file::CurrentFileContext, types::DocumentLines},
    helpers::insert_range,
};

#[derive(Debug, Clone)]
pub struct FillInMiddleStreamContext {
    prefix_at_cursor_position: String,
}

impl FillInMiddleStreamContext {
    fn new(prefix_at_cursor_position: String) -> Self {
        Self {
            prefix_at_cursor_position,
        }
    }
}

pub struct FillInMiddleCompletionAgent {
    llm_broker: Arc<LLMBroker>,
    llm_tokenizer: Arc<LLMTokenizer>,
    fill_in_middle_broker: Arc<FillInMiddleBroker>,
    editor_parsing: Arc<EditorParsing>,
    answer_mode: Arc<LLMAnswerModelBroker>,
}

#[derive(thiserror::Error, Debug)]
pub enum InLineCompletionError {
    #[error("LLM type {0} is not supported for inline completion.")]
    LLMNotSupported(LLMType),

    #[error("Language Not supported: {0}")]
    LanguageNotSupported(String),

    #[error("tokenizer formatting error: {0}")]
    LLMTokenizerError(#[from] llm_client::format::types::TokenizerError),

    #[error("tokenizer error: {0}")]
    LLMTokenizationError(#[from] LLMTokenizerError),

    #[error("No language configuration found for path: {0}")]
    NoLanguageConfiguration(String),

    #[error("Fill in middle error: {0}")]
    FillInMiddleError(#[from] llm_prompts::fim::types::FillInMiddleError),

    #[error("Missing provider keys: {0}")]
    MissingProviderKeys(LLMType),

    #[error("LLMClient error: {0}")]
    LLMClientError(#[from] llm_client::clients::types::LLMClientError),

    #[error("terminated streamed completion")]
    InlineCompletionTerminated,

    #[error("Tokenizer not found: {0}")]
    TokenizerNotFound(LLMType),

    #[error("Tokenization error: {0}")]
    TokenizationError(LLMType),

    #[error("Prefix not found for the cursor position")]
    PrefixNotFound,
}

struct InLineCompletionData {
    prefix: String,
    suffix: String,
    line: String,
}

impl FillInMiddleCompletionAgent {
    pub fn new(
        llm_broker: Arc<LLMBroker>,
        llm_tokenizer: Arc<LLMTokenizer>,
        answer_mode: Arc<LLMAnswerModelBroker>,
        fill_in_middle_broker: Arc<FillInMiddleBroker>,
        editor_parsing: Arc<EditorParsing>,
    ) -> Self {
        Self {
            llm_broker,
            llm_tokenizer,
            answer_mode,
            fill_in_middle_broker,
            editor_parsing,
        }
    }

    pub fn completion(
        &self,
        completion_request: InlineCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<InlineCompletionResponse, InLineCompletionError>> + Send>>,
        InLineCompletionError,
    > {
        // Now that we have the position, we want to create the request for the fill
        // in the middle request.
        let model_config = &completion_request.model_config;
        let fast_model = model_config.fast_model.clone();
        let temperature = model_config
            .fast_model_temperature()
            .ok_or(InLineCompletionError::LLMNotSupported(fast_model.clone()))?;
        let fast_model_api_key = model_config
            .provider_for_fast_model()
            .ok_or(InLineCompletionError::MissingProviderKeys(
                fast_model.clone(),
            ))?
            .clone();
        let model_config = self.answer_mode.get_answer_model(&fast_model);
        if let None = model_config {
            return Err(InLineCompletionError::LLMNotSupported(fast_model));
        }
        let token_limit = model_config
            .expect("if let None to hold")
            .inline_completion_tokens;
        if let None = token_limit {
            return Err(InLineCompletionError::LLMNotSupported(fast_model));
        }
        let mut token_limit = token_limit.expect("if let None to hold");

        dbg!("generating_context_start", Local::now());
        let document_lines = DocumentLines::from_file_content(&completion_request.text);

        let mut prefix = None;
        if let Some(completion_context) = completion_request.cliboard_content {
            let clipboard_context = ClipboardContext::new(
                completion_context,
                self.llm_tokenizer.clone(),
                fast_model.clone(),
                self.editor_parsing.clone(),
                completion_request.filepath.to_owned(),
            )
            .get_clipboard_context(100)?;
            match clipboard_context {
                ClipboardContextString::TruncatedToLimit(
                    clipboard_context,
                    clipboard_tokens_used,
                ) => {
                    token_limit = token_limit - clipboard_tokens_used;
                    prefix = Some(clipboard_context);
                }
                _ => {}
            }
        };

        // Now we are going to grab the current line prefix
        let cursor_prefix = Arc::new(FillInMiddleStreamContext::new(
            document_lines.prefix_at_line(completion_request.position)?,
        ));

        // Now we generate the prefix and the suffix here
        let completion_context = CurrentFileContext::new(
            completion_request.filepath,
            completion_request.position,
            token_limit as usize,
            self.llm_tokenizer.clone(),
            self.editor_parsing.clone(),
            fast_model.clone(),
        )
        .generate_context(&document_lines)?;

        let formatted_string = self.fill_in_middle_broker.format_context(
            match prefix {
                Some(prefix) => FillInMiddleRequest::new(
                    format!(
                        "{}\n{}",
                        prefix,
                        completion_context.prefix.content().to_owned()
                    ),
                    completion_context.suffix.content().to_owned(),
                ),
                None => FillInMiddleRequest::new(
                    completion_context.prefix.content().to_owned(),
                    completion_context.suffix.content().to_owned(),
                ),
            },
            &fast_model,
        )?;
        dbg!("generating_context_end", Local::now());

        let arced_document_lines = Arc::new(document_lines);

        // Now we send a request over to our provider and get a response for this
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let completion_receiver_stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(receiver).map(either::Left);
        // pin_mut!(merged_stream);

        let llm_broker = self.llm_broker.clone();
        Ok(Box::pin({
            let cursor_prefix = cursor_prefix.clone();
            // ugly, ugly, ugly, but type-safe so yay :))
            let completion = LLMBroker::stream_string_completion_owned(
                llm_broker,
                fast_model_api_key,
                LLMClientCompletionStringRequest::new(
                    fast_model.clone(),
                    formatted_string.filled.to_owned(),
                    temperature,
                    None,
                )
                // we are dumping the same eot for different models here, which
                // is fine but we can change this later
                .set_stop_words(vec![
                    "\n\n".to_owned(),
                    "```".to_owned(),
                    "<EOT>".to_owned(),
                    "</s>".to_owned(),
                    "<｜end▁of▁sentence｜>".to_owned(),
                    "<｜begin▁of▁sentence｜>".to_owned(),
                    "<step>".to_owned(),
                ])
                // we only allow for 256 tokens so we can quickly get back the response
                // and terminate if we are going through a bad request
                .set_max_tokens(256),
                vec![("event_type".to_owned(), "fill_in_middle".to_owned())]
                    .into_iter()
                    .collect(),
                sender,
            )
            .into_stream()
            .map(either::Right);

            let merged_stream = stream::select(completion_receiver_stream, completion);
            merged_stream
                .map(move |item| {
                    (
                        item,
                        arced_document_lines.clone(),
                        formatted_string.clone(),
                        cursor_prefix.clone(),
                    )
                })
                .map(
                    move |(item, document_lines, formatted_string, cursor_prefix)| match item {
                        either::Left(response) => Ok((
                            InlineCompletionResponse::new(
                                vec![InlineCompletion::new(
                                    response.answer_up_until_now().to_owned(),
                                    insert_range(
                                        completion_request.position,
                                        &document_lines,
                                        response.answer_up_until_now(),
                                    ),
                                    response.delta().map(|v| v.to_owned()),
                                )],
                                formatted_string.filled.to_owned(),
                            ),
                            cursor_prefix.clone(),
                        )),
                        either::Right(Ok(response)) => {
                            Ok((
                                InlineCompletionResponse::new(
                                    // this gets sent at the very end
                                    vec![InlineCompletion::new(
                                        response.to_owned(),
                                        insert_range(
                                            completion_request.position,
                                            &document_lines,
                                            &response,
                                        ),
                                        None,
                                    )],
                                    formatted_string.filled.to_owned(),
                                ),
                                cursor_prefix,
                            ))
                        }
                        either::Right(Err(e)) => {
                            println!("{:?}", e);
                            Err(InLineCompletionError::InlineCompletionTerminated)
                        }
                    },
                )
                .take_while(
                    |inline_completion_response| match inline_completion_response {
                        Ok((inline_completion_response, cursor_prefix)) => {
                            // Now we can check if we should still be sending the item over, and we work independently over here on a state
                            // basis and not the stream basis
                            let inserted_text = inline_completion_response
                                .completions
                                .get(0)
                                .map(|completion| completion.insert_text.to_owned());
                            if let Some(inserted_text) = inserted_text {
                                if check_terminating_condition(inserted_text) {
                                    futures::future::ready(false)
                                } else {
                                    futures::future::ready(true)
                                }
                            } else {
                                futures::future::ready(true)
                            }
                        }
                        Err(_) => futures::future::ready(false),
                    },
                )
                .map(|item| match item {
                    Ok((inline_completion, _cursor_prefix)) => Ok(inline_completion),
                    Err(e) => Err(e),
                })
        }))
    }
}

fn check_terminating_condition(inserted_text: String) -> bool {
    // first we check if the lines are, and check for opening and closing brackets
    // the patterns we will look for are: {}, [], (), <>
    let opening_brackets = vec!["{", "[", "(", "<"];
    let closing_brackets = vec!["}", "]", ")", ">"];
    let mut bracket_count = 0;
    let mut opening_bracket_detected = false;
    inserted_text.chars().into_iter().for_each(|character| {
        let character_str = character.to_string();
        if opening_brackets.contains(&character_str.as_str()) {
            bracket_count = bracket_count + 1;
            opening_bracket_detected = true;
        }
        if closing_brackets.contains(&&character_str.as_str()) {
            bracket_count = bracket_count - 1;
        }
    });
    if opening_bracket_detected && bracket_count == 0 {
        dbg!("terminating_condition_true", &inserted_text);
        true
    } else {
        dbg!("terminating_condition", &inserted_text);
        false
    }
}
