//! The llm client broker takes care of getting the right tokenizer formatter etc
//! without us having to worry about the specifics, just pass in the message and the
//! provider we take care of the rest

use crate::{
    clients::{
        ollama::OllamaClient, openai::OpenAIClient, togetherai::TogetherAIClient, types::LLMClient,
    },
    provider::{self, LLMProvider},
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
}
