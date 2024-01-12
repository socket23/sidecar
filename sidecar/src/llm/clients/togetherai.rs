use crate::llm::provider::TogetherAIProvider;
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;

use super::types::LLMClient;
use super::types::LLMClientCompletionRequest;
use super::types::LLMClientCompletionResponse;
use super::types::LLMClientError;

pub struct TogetherAIClient {
    pub client: reqwest::Client,
    pub base_url: String,
    pub provider_details: TogetherAIProvider,
}

#[derive(serde::Serialize, Debug, Clone)]
struct TogetherAIRequest {
    prompt: String,
    model: String,
    temperature: f32,
    stream_tokens: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct TogetherAIResponse {
    choices: Vec<Choice>,
    // id: String,
    // token: Token,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Choice {
    text: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Token {
    id: i32,
    text: String,
    logprob: i32,
    special: bool,
}

impl TogetherAIRequest {
    pub fn from_request(request: LLMClientCompletionRequest) -> Self {
        Self {
            prompt: request.prompt().to_owned(),
            model: request.model().to_owned(),
            temperature: request.temperature(),
            stream_tokens: true,
            frequency_penalty: request.frequency_penalty(),
        }
    }
}

impl TogetherAIClient {
    pub fn new(provider_details: TogetherAIProvider) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            base_url: "https://api.together.xyz".to_owned(),
            provider_details,
        }
    }

    pub fn inference_endpoint(&self) -> String {
        format!("{}/inference", self.base_url)
    }
}

#[async_trait]
impl LLMClient for TogetherAIClient {
    async fn completion(
        &self,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError> {
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        self.stream_completion(request, sender).await
    }

    async fn stream_completion(
        &self,
        request: LLMClientCompletionRequest,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        let model = request.model().to_owned();
        let together_ai_request = TogetherAIRequest::from_request(request);
        let mut response_stream = self
            .client
            .post(self.inference_endpoint())
            .bearer_auth(self.provider_details.api_key.to_owned())
            .json(&together_ai_request)
            .send()
            .await?
            .bytes_stream()
            .eventsource();

        let mut buffered_string = "".to_owned();
        while let Some(event) = response_stream.next().await {
            match event {
                Ok(event) => {
                    let value = serde_json::from_str::<TogetherAIResponse>(&event.data)?;
                    buffered_string = buffered_string + &value.choices[0].text;
                    sender.send(LLMClientCompletionResponse::new(
                        buffered_string.to_owned(),
                        Some(value.choices[0].text.to_owned()),
                        model.to_owned(),
                    ))?;
                }
                Err(_) => {}
            }
        }

        Ok(buffered_string)
    }
}
