//! Contains types for setting the provider for the LLM, we are going to support
//! 3 things for now:
//! - CodeStory
//! - OpenAI
//! - Ollama
//! - together.ai

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum LLMProvider {
    OpenAI(OpenAIProvider),
    TogetherAI(TogetherAIProvider),
    Ollama(OllamaProvider),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OpenAIProvider {
    pub api_key: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TogetherAIProvider {
    pub api_key: String,
    pub model_name: String,
}

impl TogetherAIProvider {
    pub fn new(api_key: String, model_name: String) -> Self {
        Self {
            api_key,
            model_name,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OllamaProvider {
    pub model_name: String,
}

impl LLMProvider {
    pub fn get_api_key(&self) -> Option<&str> {
        match self {
            LLMProvider::OpenAI(provider) => Some(&provider.api_key),
            LLMProvider::TogetherAI(provider) => Some(&provider.api_key),
            LLMProvider::Ollama(_) => None,
        }
    }

    pub fn model_name(&self) -> Option<&str> {
        match self {
            LLMProvider::OpenAI(_) => None,
            LLMProvider::TogetherAI(provider) => Some(&provider.model_name),
            LLMProvider::Ollama(provider) => Some(&provider.model_name),
        }
    }
}
