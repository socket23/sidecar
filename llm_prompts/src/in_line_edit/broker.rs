use std::collections::HashMap;

use llm_client::clients::types::LLMType;

use super::{
    mistral::MistralLineEditPrompt,
    openai::OpenAILineEditPrompt,
    types::{InLineEditPrompt, InLineEditPromptError},
};

pub struct InLineEditPromptBroker {
    prompt_generators: HashMap<LLMType, Box<dyn InLineEditPrompt>>,
}

impl InLineEditPromptBroker {
    pub fn new() -> Self {
        let broker = Self {
            prompt_generators: HashMap::new(),
        };
        broker
            .insert_prompt_generator(LLMType::GPT3_5_16k, Box::new(OpenAILineEditPrompt::new()))
            .insert_prompt_generator(LLMType::Gpt4, Box::new(OpenAILineEditPrompt::new()))
            .insert_prompt_generator(LLMType::Gpt4_32k, Box::new(OpenAILineEditPrompt::new()))
            .insert_prompt_generator(
                LLMType::MistralInstruct,
                Box::new(MistralLineEditPrompt::new()),
            )
    }

    pub fn insert_prompt_generator(
        mut self,
        llm_type: LLMType,
        prompt_generator: Box<dyn InLineEditPrompt>,
    ) -> Self {
        self.prompt_generators.insert(llm_type, prompt_generator);
        self
    }

    pub fn get_prompt_generator(
        &self,
        llm_type: &LLMType,
    ) -> Result<&Box<dyn InLineEditPrompt>, InLineEditPromptError> {
        self.prompt_generators
            .get(llm_type)
            .ok_or(InLineEditPromptError::ModelNotSupported)
    }
}
