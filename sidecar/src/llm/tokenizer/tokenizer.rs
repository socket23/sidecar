//! We are going to run the various tokenizers here, we also make sure to run
//! the toknizer in a different thread here, because its important that we
//! don't block the main thread from working

use std::collections::HashMap;
use std::str::FromStr;

use thiserror::Error;
use tiktoken_rs::ChatCompletionRequestMessage;
use tokenizers::Tokenizer;

use crate::llm::clients::types::{LLMClientMessage, LLMClientRole, LLMType};

pub struct LLMTokenizer {
    pub tokenizers: HashMap<LLMType, Tokenizer>,
}

#[derive(Error, Debug)]
pub enum LLMTokenizerError {
    #[error("Tokenizer not found for model {0}")]
    TokenizerNotFound(LLMType),

    #[error("Tokenizer error: {0}")]
    TokenizerError(String),

    #[error("error from tokenizer crate: {0}")]
    TokenizerCrateError(#[from] tokenizers::Error),

    #[error("anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

pub enum LLMTokenizerInput {
    Prompt(String),
    Messages(Vec<LLMClientMessage>),
}

impl LLMTokenizer {
    fn to_openai_tokenizer(&self, model: &LLMType) -> Option<String> {
        match model {
            LLMType::GPT3_5_16k => Some("gpt-3.5-turbo-16k-0613".to_owned()),
            LLMType::Gpt4 => Some("gpt-4-0613".to_owned()),
            LLMType::Gpt4Turbo => Some("gpt-4-1106-preview".to_owned()),
            LLMType::Gpt4_32k => Some("gpt-4-32k-0613".to_owned()),
            _ => None,
        }
    }

    pub fn tokenizer(
        &self,
        model: &LLMType,
        input: LLMTokenizerInput,
    ) -> Result<usize, LLMTokenizerError> {
        match input {
            LLMTokenizerInput::Prompt(prompt) => self.count_tokens_using_tokenizer(model, &prompt),
            LLMTokenizerInput::Messages(messages) => {
                // we can't send messages directly to the tokenizer, we have to
                // either make it a message or its an openai prompt in which case
                // its fine
                // so we are going to return an error if its not openai
                if model.is_openai() {
                    // we can use the openai tokenizer
                    let model = self.to_openai_tokenizer(model);
                    match model {
                        Some(model) => Ok(tiktoken_rs::num_tokens_from_messages(
                            &model,
                            messages
                                .into_iter()
                                .map(|message| {
                                    let role = message.role();
                                    let content = message.content();
                                    match role {
                                        LLMClientRole::User => ChatCompletionRequestMessage {
                                            role: "user".to_owned(),
                                            content: Some(content.to_owned()),
                                            name: None,
                                            function_call: None,
                                        },
                                        LLMClientRole::Assistant => ChatCompletionRequestMessage {
                                            role: "assistant".to_owned(),
                                            content: Some(content.to_owned()),
                                            name: None,
                                            function_call: None,
                                        },
                                        LLMClientRole::System => ChatCompletionRequestMessage {
                                            role: "system".to_owned(),
                                            content: Some(content.to_owned()),
                                            name: None,
                                            function_call: None,
                                        },
                                    }
                                })
                                .collect::<Vec<_>>()
                                .as_slice(),
                        )?),
                        None => Err(LLMTokenizerError::TokenizerError(
                            "Only openai models are supported for messages".to_owned(),
                        )),
                    }
                } else {
                    // we can't use the openai tokenizer
                    Err(LLMTokenizerError::TokenizerError(
                        "Only openai models are supported for messages".to_owned(),
                    ))
                }
            }
        }
    }

    pub fn count_tokens_using_tokenizer(
        &self,
        model: &LLMType,
        prompt: &str,
    ) -> Result<usize, LLMTokenizerError> {
        let tokenizer = self.tokenizers.get(model);
        match tokenizer {
            Some(tokenizer) => {
                // Now over here we will try to figure out how to pass the
                // values around
                let results = tokenizer.encode_batch(vec![prompt], false);
                if let Ok(results) = results {
                    match results.first() {
                        Some(result) => Ok(result.len()),
                        None => Err(LLMTokenizerError::TokenizerError(
                            "No results found".to_owned(),
                        )),
                    }
                } else {
                    Err(LLMTokenizerError::TokenizerError(
                        "Failed to encode batch".to_owned(),
                    ))
                }
            }
            None => {
                return Err(LLMTokenizerError::TokenizerNotFound(model.clone()));
            }
        }
    }

    pub fn load_tokenizer(&mut self, model: &LLMType) -> Result<(), LLMTokenizerError> {
        let tokenizer = match model {
            LLMType::MistralInstruct => {
                let config = include_str!("configs/mistral.json");
                let loaded_tokenizer = Tokenizer::from_str(&config);
                Some(Tokenizer::from_str(config)?)
            }
            LLMType::Mixtral => {
                let config = include_str!("configs/mixtral.json");
                Some(Tokenizer::from_str(config)?)
            }
            _ => None,
        };
        if let Some(tokenizer) = tokenizer {
            self.tokenizers.insert(model.clone(), tokenizer);
        }
        Ok(())
    }
}
