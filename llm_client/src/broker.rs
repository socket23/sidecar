//! The llm client broker takes care of getting the right tokenizer formatter etc
//! without us having to worry about the specifics, just pass in the message and the
//! provider we take care of the rest

use std::collections::HashMap;

use crate::{
    clients::{
        ollama::OllamaClient,
        openai::OpenAIClient,
        togetherai::TogetherAIClient,
        types::{
            LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse, LLMClientError,
        },
    },
    provider::{self, LLMProvider, LLMProviderAPIKeys},
};

pub struct LLMBroker {
    pub providers: HashMap<LLMProvider, Box<dyn LLMClient>>,
}

impl LLMBroker {
    pub fn new() -> Self {
        let mut broker = Self {
            providers: HashMap::new(),
        };
        broker
            .add_provider(LLMProvider::OpenAI, Box::new(OpenAIClient::new()))
            .add_provider(LLMProvider::Ollama, Box::new(OllamaClient::new()))
            .add_provider(LLMProvider::TogetherAI, Box::new(TogetherAIClient::new()))
    }

    pub fn add_provider(mut self, provider: LLMProvider, client: Box<dyn LLMClient>) -> Self {
        self.providers.insert(provider, client);
        self
    }

    pub async fn stream_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
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
}
