//! Contains types for setting the provider for the LLM, we are going to support
//! 3 things for now:
//! - CodeStory
//! - OpenAI
//! - Ollama
//! - Azure
//! - together.ai

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq)]
pub enum LLMProvider {
    OpenAI,
    TogetherAI,
    Ollama,
    Azure,
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

    // Gets the relevant key from the llm provider
    pub fn key(&self, llm_provider: &LLMProvider) -> Option<Self> {
        match llm_provider {
            LLMProvider::OpenAI => {
                if let LLMProviderAPIKeys::OpenAI(key) = self {
                    Some(LLMProviderAPIKeys::OpenAI(key.clone()))
                } else {
                    None
                }
            }
            LLMProvider::TogetherAI => {
                if let LLMProviderAPIKeys::TogetherAI(key) = self {
                    Some(LLMProviderAPIKeys::TogetherAI(key.clone()))
                } else {
                    None
                }
            }
            LLMProvider::Ollama => {
                if let LLMProviderAPIKeys::Ollama(key) = self {
                    Some(LLMProviderAPIKeys::Ollama(key.clone()))
                } else {
                    None
                }
            }
            LLMProvider::Azure => {
                if let LLMProviderAPIKeys::OpenAIAzureConfig(key) = self {
                    Some(LLMProviderAPIKeys::OpenAIAzureConfig(key.clone()))
                } else {
                    None
                }
            }
        }
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
    pub api_version: String,
}
