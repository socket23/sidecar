//! Which LLMs do we support

use std::fmt::Display;

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, clap::ValueEnum)]
pub enum LLMType {
    OpenAI,
    Mistral,
}

impl Display for LLMType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match self {
            LLMType::OpenAI => "open-ai".to_owned(),
            LLMType::Mistral => "mistral".to_owned(),
        };
        f.write_str(&desc)
    }
}

impl From<String> for LLMType {
    fn from(value: String) -> Self {
        if value == "mistral" {
            LLMType::Mistral
        } else {
            LLMType::OpenAI
        }
    }
}

impl Default for LLMType {
    fn default() -> Self {
        LLMType::OpenAI
    }
}

#[derive(Debug, Clone)]
pub struct LLMCustomConfig {
    pub llm: LLMType,
    pub endpoint: String,
}

impl LLMCustomConfig {
    pub fn openai() -> Self {
        Self {
            llm: LLMType::OpenAI,
            endpoint: "https://api.openai.com/v1".to_owned(),
        }
    }

    pub fn mistral(endpoint: String) -> Self {
        Self {
            llm: LLMType::Mistral,
            endpoint,
        }
    }

    pub fn non_openai_endpoint(&self) -> Option<&str> {
        match &self.llm {
            &LLMType::OpenAI => None,
            &LLMType::Mistral => Some(&self.endpoint),
        }
    }
}
