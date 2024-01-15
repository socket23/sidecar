//! Contains types for setting the provider for the LLM, we are going to support
//! 3 things for now:
//! - CodeStory
//! - OpenAI
//! - Ollama
//! - together.ai

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum LLMProvider {
    OpenAI,
    TogetherAI,
    Ollama,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum LLMProviderAPIKeys {
    OpenAI(OpenAIProvider),
    TogetherAI(TogetherAIProvider),
    Ollama(OllamaProvider),
    OpenAIAzureConfig(AzureConfig),
}

impl LLMProviderAPIKeys {
    pub fn is_openai(&self) -> bool {
        matches!(self, LLMProviderAPIKeys::OpenAI(_))
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OpenAIProvider {
    pub api_key: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TogetherAIProvider {
    pub api_key: String,
}

impl TogetherAIProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OllamaProvider {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AzureConfig {
    pub deployment_id: String,
    pub api_base: String,
    pub api_key: String,
}
