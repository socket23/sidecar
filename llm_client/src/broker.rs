//! The llm client broker takes care of getting the right tokenizer formatter etc
//! without us having to worry about the specifics, just pass in the message and the
//! provider we take care of the rest

use std::collections::HashMap;

use futures::future::Either;

use crate::{
    clients::{
        ollama::OllamaClient,
        openai::OpenAIClient,
        togetherai::TogetherAIClient,
        types::{
            LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse,
            LLMClientCompletionStringRequest, LLMClientError,
        },
    },
    provider::{LLMProvider, LLMProviderAPIKeys},
};

pub struct LLMBroker {
    pub providers: HashMap<LLMProvider, Box<dyn LLMClient + Send + Sync>>,
}

pub type LLMBrokerResponse = Result<String, LLMClientError>;

impl LLMBroker {
    pub fn new() -> Self {
        let broker = Self {
            providers: HashMap::new(),
        };
        broker
            .add_provider(LLMProvider::OpenAI, Box::new(OpenAIClient::new()))
            .add_provider(LLMProvider::Ollama, Box::new(OllamaClient::new()))
            .add_provider(LLMProvider::TogetherAI, Box::new(TogetherAIClient::new()))
    }

    pub fn add_provider(
        mut self,
        provider: LLMProvider,
        client: Box<dyn LLMClient + Send + Sync>,
    ) -> Self {
        self.providers.insert(provider, client);
        self
    }

    pub async fn stream_answer(
        &self,
        api_key: LLMProviderAPIKeys,
        request: Either<LLMClientCompletionRequest, LLMClientCompletionStringRequest>,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> LLMBrokerResponse {
        match request {
            Either::Left(request) => self.stream_completion(api_key, request, sender).await,
            Either::Right(request) => {
                self.stream_string_completion(api_key, request, sender)
                    .await
            }
        }
    }

    pub async fn stream_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> LLMBrokerResponse {
        let provider_type = match &api_key {
            LLMProviderAPIKeys::Ollama(_) => LLMProvider::Ollama,
            LLMProviderAPIKeys::OpenAI(_) => LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAIAzureConfig(_) => LLMProvider::OpenAI,
            LLMProviderAPIKeys::TogetherAI(_) => LLMProvider::TogetherAI,
        };
        let provider = self.providers.get(&provider_type);
        if let Some(provider) = provider {
            provider.stream_completion(api_key, request, sender).await
        } else {
            Err(LLMClientError::UnSupportedModel)
        }
    }

    pub async fn stream_string_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> LLMBrokerResponse {
        let provider_type = match &api_key {
            LLMProviderAPIKeys::Ollama(_) => LLMProvider::Ollama,
            LLMProviderAPIKeys::OpenAI(_) => LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAIAzureConfig(_) => LLMProvider::OpenAI,
            LLMProviderAPIKeys::TogetherAI(_) => LLMProvider::TogetherAI,
        };
        let provider = self.providers.get(&provider_type);
        if let Some(provider) = provider {
            provider
                .stream_prompt_completion(api_key, request, sender)
                .await
        } else {
            Err(LLMClientError::UnSupportedModel)
        }
    }
}
