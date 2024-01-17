//! The various interfaces for prompt declaration we have for the in line agent
//! chat. We take care to send the data here properly (after filtering/reranking etc)
//! and let the LLM decide what we want to do with it

use llm_client::clients::types::LLMClientMessage;

pub struct InLineFixRequest {
    above: Option<String>,
    below: Option<String>,
    in_range: Option<String>,
    diagnostics_prompts: Vec<String>,
    language: String,
}

impl InLineFixRequest {
    pub fn above(&self) -> Option<&String> {
        self.above.as_ref()
    }

    pub fn below(&self) -> Option<&String> {
        self.below.as_ref()
    }

    pub fn in_range(&self) -> Option<&String> {
        self.in_range.as_ref()
    }

    pub fn diagnostics_prompts(&self) -> &[String] {
        &self.diagnostics_prompts
    }

    pub fn language(&self) -> &str {
        &self.language
    }
}

pub struct InLineEditRequest {
    above: Option<String>,
    below: Option<String>,
    in_range: Option<String>,
    user_query: String,
    file_path: String,
    /// The extra symbols or data which the user has passed as reference
    extra_data: Vec<String>,
    language: String,
}

impl InLineEditRequest {
    pub fn above(&self) -> Option<&String> {
        self.above.as_ref()
    }

    pub fn below(&self) -> Option<&String> {
        self.below.as_ref()
    }

    pub fn in_range(&self) -> Option<&String> {
        self.in_range.as_ref()
    }

    pub fn user_query(&self) -> &str {
        &self.user_query
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn extra_data(&self) -> &[String] {
        &self.extra_data
    }

    pub fn language(&self) -> &str {
        &self.language
    }
}

impl InLineEditRequest {
    pub fn new(
        above: Option<String>,
        below: Option<String>,
        in_range: Option<String>,
        user_query: String,
        file_path: String,
        extra_data: Vec<String>,
        language: String,
    ) -> Self {
        Self {
            above,
            below,
            in_range,
            user_query,
            file_path,
            extra_data,
            language,
        }
    }
}

/// We might end up calling the chat or the completion endpoint for a LLM,
/// its important that we support both
#[derive(Debug)]
pub enum InLinePromptResponse {
    Completion(String),
    Chat(Vec<LLMClientMessage>),
}

impl InLinePromptResponse {
    pub fn completion(completion: String) -> Self {
        InLinePromptResponse::Completion(completion)
    }

    pub fn get_completion(self) -> Option<String> {
        if let InLinePromptResponse::Completion(completion) = self {
            Some(completion)
        } else {
            None
        }
    }
}

/// Should we send context here as the above, below and in line context, or do we
/// just send the data as it is?
pub trait InLineEditPrompt {
    fn inline_edit(&self, request: InLineEditRequest) -> InLinePromptResponse;

    fn inline_fix(&self, request: InLineFixRequest) -> InLinePromptResponse;
}

/// The error type which we will return if we do not support that model yet
#[derive(thiserror::Error, Debug)]
pub enum InLineEditPromptError {
    #[error("Model not supported yet")]
    ModelNotSupported,
}
