use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum LLMType {
    Mixtral,
    MistralInstruct,
    Gpt4,
    GPT3_5_16k,
    Gpt4_32k,
    Gpt4Turbo,
    Custom(String),
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
}

#[async_trait]
pub trait LLMClient {
    async fn stream_completion(
        &self,
        request: LLMClientCompletionRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError>;

    async fn completion(
        &self,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError>;
}
