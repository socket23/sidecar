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

impl LLMClientConfig {
    pub fn provider_for_slow_model(&self) -> Option<&LLMProviderAPIKeys> {
        // we first need to get the model configuration for the slow model
        // which will give us the model and the context around it
        let model = self.models.get(&self.fast_model);
        if let None = model {
            return None;
        }
        let model = model.expect("is_none above to hold");
        let provider = &model.provider;
        // get the related provider if its present
        self.providers.iter().find(|p| p.key(provider).is_some())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Model {
    pub name: LLMType,
    pub context_length: u32,
    pub temperature: f32,
    pub provider: LLMProvider,
}
