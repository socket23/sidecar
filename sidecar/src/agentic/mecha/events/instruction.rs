use llm_client::{
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

pub struct MechaInstructionEvent {
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    steps: Vec<String>,
}

impl MechaInstructionEvent {
    pub fn new(
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        steps: Vec<String>,
    ) -> Self {
        Self {
            llm,
            provider,
            api_keys,
            steps,
        }
    }

    pub fn steps(&self) -> &[String] {
        self.steps.as_slice()
    }
}
