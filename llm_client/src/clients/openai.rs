//! Client which can help us talk to openai

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs,
        CreateChatCompletionRequestArgs, Role,
    },
    Client,
};
use async_trait::async_trait;
use futures::StreamExt;

use crate::provider::OpenAIProvider;

use super::types::{
    LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse, LLMClientError,
    LLMClientMessage, LLMClientRole, LLMType,
};

pub struct OpenAIClient {
    client: Client<OpenAIConfig>,
}

impl OpenAIClient {
    pub fn new(provider: OpenAIProvider) -> Self {
        let config = OpenAIConfig::new().with_api_key(provider.api_key);
        Self {
            client: Client::with_config(config),
        }
    }

    pub fn model(&self, model: &LLMType) -> Option<String> {
        match model {
            LLMType::GPT3_5_16k => Some("gpt-3.5-turbo-16k-0613".to_owned()),
            LLMType::Gpt4 => Some("gpt-4-0613".to_owned()),
            LLMType::Gpt4Turbo => Some("gpt-4-1106-preview".to_owned()),
            LLMType::Gpt4_32k => Some("gpt-4-32k-0613".to_owned()),
            _ => None,
        }
    }

    pub fn messages(
        &self,
        messages: &[LLMClientMessage],
    ) -> Result<Vec<ChatCompletionRequestMessage>, LLMClientError> {
        let formatted_messages = messages
            .into_iter()
            .map(|message| {
                let role = message.role();
                match role {
                    LLMClientRole::User => ChatCompletionRequestMessageArgs::default()
                        .role(Role::User)
                        .content(message.content().to_owned())
                        .build()
                        .map_err(|e| LLMClientError::OpenAPIError(e)),
                    LLMClientRole::System => ChatCompletionRequestMessageArgs::default()
                        .role(Role::System)
                        .content(message.content().to_owned())
                        .build()
                        .map_err(|e| LLMClientError::OpenAPIError(e)),
                    LLMClientRole::Assistant => ChatCompletionRequestMessageArgs::default()
                        .role(Role::Assistant)
                        .content(message.content().to_owned())
                        .build()
                        .map_err(|e| LLMClientError::OpenAPIError(e)),
                }
            })
            .collect::<Vec<_>>();
        formatted_messages
            .into_iter()
            .collect::<Result<Vec<ChatCompletionRequestMessage>, LLMClientError>>()
    }
}

#[async_trait]
impl LLMClient for OpenAIClient {
    async fn stream_completion(
        &self,
        request: LLMClientCompletionRequest,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        let model = self.model(request.model());
        if model.is_none() {
            return Err(LLMClientError::UnSupportedModel);
        }
        let model = model.unwrap();
        let messages = self.messages(request.messages())?;
        let mut request_builder_args = CreateChatCompletionRequestArgs::default();
        let mut request_builder = request_builder_args
            .model(model.to_owned())
            .messages(messages)
            .temperature(request.temperature())
            .stream(true);
        if let Some(frequency_penalty) = request.frequency_penalty() {
            request_builder = request_builder.frequency_penalty(frequency_penalty);
        }
        let request = request_builder.build()?;
        let mut buffer = String::new();
        let mut stream = self.client.chat().create_stream(request).await?;

        while let Some(response) = stream.next().await {
            match response {
                Ok(response) => {
                    let response = response
                        .choices
                        .get(0)
                        .ok_or(LLMClientError::FailedToGetResponse)?;
                    let text = response.delta.content.to_owned();
                    if let Some(text) = text {
                        buffer.push_str(&text);
                        let _ = sender.send(LLMClientCompletionResponse::new(
                            buffer.to_owned(),
                            Some(text),
                            model.to_owned(),
                        ));
                    }
                }
                Err(err) => {
                    dbg!(err);
                    break;
                }
            }
        }
        Ok(buffer)
    }

    async fn completion(
        &self,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError> {
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let result = self.stream_completion(request, sender).await?;
        Ok(result)
    }
}
