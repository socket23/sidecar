use async_trait::async_trait;
use std::fmt;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

use crate::provider::{LLMProvider, LLMProviderAPIKeys};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
pub enum LLMType {
    Mixtral,
    MistralInstruct,
    Gpt4,
    GPT3_5_16k,
    Gpt4_32k,
    Gpt4Turbo,
    Custom(String),
}

impl LLMType {
    pub fn is_openai(&self) -> bool {
        matches!(
            self,
            LLMType::Gpt4 | LLMType::GPT3_5_16k | LLMType::Gpt4_32k | LLMType::Gpt4Turbo
        )
    }
}

impl fmt::Display for LLMType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMType::Mixtral => write!(f, "Mixtral"),
            LLMType::MistralInstruct => write!(f, "MistralInstruct"),
            LLMType::Gpt4 => write!(f, "Gpt4"),
            LLMType::GPT3_5_16k => write!(f, "GPT3_5_16k"),
            LLMType::Gpt4_32k => write!(f, "Gpt4_32k"),
            LLMType::Gpt4Turbo => write!(f, "Gpt4Turbo"),
            LLMType::Custom(s) => write!(f, "Custom({})", s),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum LLMClientRole {
    System,
    User,
    Assistant,
}

impl LLMClientRole {
    pub fn is_system(&self) -> bool {
        matches!(self, LLMClientRole::System)
    }

    pub fn is_user(&self) -> bool {
        matches!(self, LLMClientRole::User)
    }

    pub fn is_assistant(&self) -> bool {
        matches!(self, LLMClientRole::Assistant)
    }
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct LLMClientMessage {
    role: LLMClientRole,
    message: String,
}

impl LLMClientMessage {
    pub fn new(role: LLMClientRole, message: String) -> Self {
        Self { role, message }
    }

    pub fn user(message: String) -> Self {
        Self::new(LLMClientRole::User, message)
    }

    pub fn assistant(message: String) -> Self {
        Self::new(LLMClientRole::Assistant, message)
    }

    pub fn content(&self) -> &str {
        &self.message
    }

    pub fn role(&self) -> &LLMClientRole {
        &self.role
    }
}

pub struct LLMClientCompletionRequest {
    model: LLMType,
    messages: Vec<LLMClientMessage>,
    temperature: f32,
    frequency_penalty: Option<f32>,
}

impl LLMClientCompletionRequest {
    pub fn new(
        model: LLMType,
        messages: Vec<LLMClientMessage>,
        temperature: f32,
        frequency_penalty: Option<f32>,
    ) -> Self {
        Self {
            model,
            messages,
            temperature,
            frequency_penalty,
        }
    }

    pub fn messages(&self) -> &[LLMClientMessage] {
        self.messages.as_slice()
    }

    pub fn temperature(&self) -> f32 {
        self.temperature
    }

    pub fn frequency_penalty(&self) -> Option<f32> {
        self.frequency_penalty
    }

    pub fn model(&self) -> &LLMType {
        &self.model
    }
}

pub struct LLMClientCompletionResponse {
    answer_up_until_now: String,
    delta: Option<String>,
    model: String,
}

impl LLMClientCompletionResponse {
    pub fn new(answer_up_until_now: String, delta: Option<String>, model: String) -> Self {
        Self {
            answer_up_until_now,
            delta,
            model,
        }
    }
}

#[derive(Error, Debug)]
pub enum LLMClientError {
    #[error("Failed to get response from LLM")]
    FailedToGetResponse,

    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("serde failed: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("send error over channel: {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<LLMClientCompletionResponse>),

    #[error("unsupported model")]
    UnSupportedModel,

    #[error("OpenAI api error: {0}")]
    OpenAPIError(#[from] async_openai::error::OpenAIError),

    #[error("Wrong api key type")]
    WrongAPIKeyType,
}

#[async_trait]
pub trait LLMClient {
    fn client(&self) -> &LLMProvider;

    async fn stream_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError>;

    async fn completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError>;
}
