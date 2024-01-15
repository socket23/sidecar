//! Contains the types for model selection which we want to use

use llm_client::{
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LLMClientConfig {
    pub slow_model: LLMType,
    pub fast_model: LLMType,
    pub models: HashMap<LLMType, Model>,
    pub providers: Vec<LLMProviderAPIKeys>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Model {
    pub name: LLMType,
    pub context_length: u32,
    pub temperature: f32,
    pub provider: LLMProvider,
}
