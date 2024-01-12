use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

pub struct LLMClientCompletionRequest {
    model: String,
    prompt: String,
    temperature: f32,
    frequency_penalty: Option<f32>,
}

impl LLMClientCompletionRequest {
    pub fn new(
        model: String,
        prompt: String,
        temperature: f32,
        frequency_penalty: Option<f32>,
    ) -> Self {
        Self {
            model,
            prompt,
            temperature,
            frequency_penalty,
        }
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn temperature(&self) -> f32 {
        self.temperature
    }

    pub fn frequency_penalty(&self) -> Option<f32> {
        self.frequency_penalty
    }

    pub fn model(&self) -> &str {
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
