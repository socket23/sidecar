//! The llm client broker takes care of getting the right tokenizer formatter etc
//! without us having to worry about the specifics, just pass in the message and the
//! provider we take care of the rest

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use either::Either;
use futures::{stream, FutureExt, StreamExt};
use sqlx::SqlitePool;

use crate::{
    clients::{
        anthropic::AnthropicClient,
        codestory::CodeStoryClient,
        fireworks::FireworksAIClient,
        lmstudio::LMStudioClient,
        ollama::OllamaClient,
        openai::OpenAIClient,
        openai_compatible::OpenAICompatibleClient,
        togetherai::TogetherAIClient,
        types::{
            LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse,
            LLMClientCompletionStringRequest, LLMClientError,
        },
    },
    config::LLMBrokerConfiguration,
    provider::{CodeStoryLLMTypes, LLMProvider, LLMProviderAPIKeys},
    sqlite,
};

pub type SqlDb = Arc<SqlitePool>;

pub struct LLMBroker {
    pub providers: HashMap<LLMProvider, Box<dyn LLMClient + Send + Sync>>,
    db: SqlDb,
}

pub type LLMBrokerResponse = Result<String, LLMClientError>;

impl LLMBroker {
    pub async fn new(config: LLMBrokerConfiguration) -> Result<Self, LLMClientError> {
        let sqlite = Arc::new(sqlite::init(config).await?);
        let broker = Self {
            providers: HashMap::new(),
            db: sqlite,
        };
        Ok(broker
            .add_provider(LLMProvider::OpenAI, Box::new(OpenAIClient::new()))
            .add_provider(LLMProvider::Ollama, Box::new(OllamaClient::new()))
            .add_provider(LLMProvider::TogetherAI, Box::new(TogetherAIClient::new()))
            .add_provider(LLMProvider::LMStudio, Box::new(LMStudioClient::new()))
            .add_provider(
                LLMProvider::OpenAICompatible,
                Box::new(OpenAICompatibleClient::new()),
            )
            .add_provider(
                LLMProvider::CodeStory(CodeStoryLLMTypes { llm_type: None }),
                Box::new(CodeStoryClient::new(
                    "https://codestory-provider-dot-anton-390822.ue.r.appspot.com",
                )),
            )
            .add_provider(LLMProvider::FireworksAI, Box::new(FireworksAIClient::new()))
            .add_provider(LLMProvider::Anthropic, Box::new(AnthropicClient::new())))
    }

    pub fn add_provider(
        mut self,
        provider: LLMProvider,
        client: Box<dyn LLMClient + Send + Sync>,
    ) -> Self {
        self.providers.insert(provider, client);
        self
    }

    pub async fn stream_answer(
        &self,
        api_key: LLMProviderAPIKeys,
        provider: LLMProvider,
        request: Either<LLMClientCompletionRequest, LLMClientCompletionStringRequest>,
        metadata: HashMap<String, String>,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> LLMBrokerResponse {
        match request {
            Either::Left(request) => {
                self.stream_completion(api_key, request, provider, metadata, sender)
                    .await
            }
            Either::Right(request) => {
                self.stream_string_completion(api_key, request, metadata, sender)
                    .await
            }
        }
    }

    pub async fn stream_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        provider: LLMProvider,
        metadata: HashMap<String, String>,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> LLMBrokerResponse {
        let api_key = api_key
            .key(&provider)
            .ok_or(LLMClientError::UnSupportedModel)?;
        let provider_type = match &api_key {
            LLMProviderAPIKeys::Ollama(_) => LLMProvider::Ollama,
            LLMProviderAPIKeys::OpenAI(_) => LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAIAzureConfig(_) => LLMProvider::OpenAI,
            LLMProviderAPIKeys::TogetherAI(_) => LLMProvider::TogetherAI,
            LLMProviderAPIKeys::LMStudio(_) => LLMProvider::LMStudio,
            LLMProviderAPIKeys::CodeStory => {
                LLMProvider::CodeStory(CodeStoryLLMTypes { llm_type: None })
            }
            LLMProviderAPIKeys::OpenAICompatible(_) => LLMProvider::OpenAICompatible,
            LLMProviderAPIKeys::Anthropic(_) => LLMProvider::Anthropic,
            LLMProviderAPIKeys::FireworksAI(_) => LLMProvider::FireworksAI,
        };
        let provider = self.providers.get(&provider_type);
        if let Some(provider) = provider {
            let result = provider
                .stream_completion(api_key, request.clone(), sender)
                .await;
            if let Ok(result) = result.as_ref() {
                // we write the inputs to the DB so we can keep track of the inputs
                // and the result provided by the LLM
                let llm_type = request.model();
                let temperature = request.temperature();
                let str_metadata = serde_json::to_string(&metadata).unwrap_or_default();
                let llm_type_str = serde_json::to_string(&llm_type)?;
                let messages = serde_json::to_string(&request.messages())?;
                let mut tx = self
                    .db
                    .begin()
                    .await
                    .map_err(|_e| LLMClientError::FailedToStoreInDB)?;
                let _ = sqlx::query! {
                    r#"
                    INSERT INTO llm_data (chat_messages, response, llm_type, temperature, max_tokens, event_type)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    "#,
                    messages,
                    result,
                    llm_type_str,
                    temperature,
                    -1,
                    str_metadata,
                }.execute(&mut *tx).await?;
                let _ = tx
                    .commit()
                    .await
                    .map_err(|_e| LLMClientError::FailedToStoreInDB)?;
            }
            result
        } else {
            Err(LLMClientError::UnSupportedModel)
        }
    }

    // TODO(skcd): Debug this part of the code later on, cause we have
    // some bugs around here about the new line we are sending over
    pub async fn stream_string_completion_owned(
        value: Arc<Self>,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        metadata: HashMap<String, String>,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> LLMBrokerResponse {
        let (sender_channel, receiver) = tokio::sync::mpsc::unbounded_channel();
        let receiver_stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(receiver).map(either::Right);
        let result = value
            .stream_string_completion(api_key, request, metadata, sender_channel)
            .into_stream()
            .map(either::Left);
        let mut final_result = None;
        struct RunningAnswer {
            answer_up_until_now: String,
            running_line: String,
        }
        let running_line = Arc::new(Mutex::new(RunningAnswer {
            answer_up_until_now: "".to_owned(),
            running_line: "".to_owned(),
        }));
        stream::select(receiver_stream, result)
            .map(|element| (element, running_line.clone()))
            .for_each(|(element, running_line)| {
                match element {
                    either::Right(item) => {
                        let delta = item.delta().map(|delta| delta.to_owned());
                        let answer_until_now = item.get_answer_up_until_now();
                        if let Ok(mut current_running_line) = running_line.lock() {
                            if let Some(delta) = delta {
                                current_running_line.running_line.push_str(&delta);
                            }
                            while let Some(new_line_index) =
                                current_running_line.running_line.find('\n')
                            {
                                let line =
                                    current_running_line.running_line[..new_line_index].to_owned();
                                let mut current_answer = current_running_line
                                    .answer_up_until_now
                                    .clone()
                                    .lines()
                                    .into_iter()
                                    .map(|line| line.to_owned())
                                    .chain(vec![line.to_owned()])
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                let _ = sender.send(LLMClientCompletionResponse::new(
                                    current_answer + "\n",
                                    Some(line.to_owned() + "\n"),
                                    "parsing_model".to_owned(),
                                ));
                                // add the new line and the \n
                                current_running_line.answer_up_until_now.push_str(&line);
                                current_running_line.answer_up_until_now.push_str("\n");

                                // drain the running line
                                current_running_line.running_line.drain(..=new_line_index);
                            }
                            // current_running_line.answer_up_until_now = answer_until_now;
                        }
                    }
                    either::Left(item) => {
                        final_result = Some(item);
                    }
                };
                futures::future::ready(())
            })
            .await;

        if let Ok(current_running_line) = running_line.lock() {
            let _ = sender.send(LLMClientCompletionResponse::new(
                current_running_line.answer_up_until_now.to_owned(),
                Some(current_running_line.running_line.to_owned()),
                "parsing_model".to_owned(),
            ));
        }
        final_result.ok_or(LLMClientError::FailedToGetResponse)?
    }

    pub async fn stream_string_completion<'a>(
        &'a self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        metadata: HashMap<String, String>,
        sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
    ) -> LLMBrokerResponse {
        let provider_type = match &api_key {
            LLMProviderAPIKeys::Ollama(_) => LLMProvider::Ollama,
            LLMProviderAPIKeys::OpenAI(_) => LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAIAzureConfig(_) => LLMProvider::OpenAI,
            LLMProviderAPIKeys::TogetherAI(_) => LLMProvider::TogetherAI,
            LLMProviderAPIKeys::LMStudio(_) => LLMProvider::LMStudio,
            LLMProviderAPIKeys::CodeStory => {
                LLMProvider::CodeStory(CodeStoryLLMTypes { llm_type: None })
            }
            LLMProviderAPIKeys::OpenAICompatible(_) => LLMProvider::OpenAICompatible,
            LLMProviderAPIKeys::Anthropic(_) => LLMProvider::Anthropic,
            LLMProviderAPIKeys::FireworksAI(_) => LLMProvider::FireworksAI,
        };
        let provider = self.providers.get(&provider_type);
        if let Some(provider) = provider {
            let result = provider
                .stream_prompt_completion(api_key, request.clone(), sender)
                .await;
            if let Ok(result) = result.as_ref() {
                // we write the inputs to the DB so we can keep track of the inputs
                // and the result provided by the LLM
                let llm_type = request.model();
                let temperature = request.temperature();
                let str_metadata = serde_json::to_string(&metadata).unwrap_or_default();
                let llm_type_str = serde_json::to_string(&llm_type)?;
                let prompt = request.prompt();
                let mut tx = self
                    .db
                    .begin()
                    .await
                    .map_err(|_e| LLMClientError::FailedToStoreInDB)?;
                let _ = sqlx::query! {
                    r#"
                    INSERT INTO llm_data (prompt, response, llm_type, temperature, max_tokens, event_type)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    "#,
                    prompt,
                    result,
                    llm_type_str,
                    temperature,
                    -1,
                    str_metadata,
                }.execute(&mut *tx).await?;
                let _ = tx
                    .commit()
                    .await
                    .map_err(|_e| LLMClientError::FailedToStoreInDB)?;
            }
            result
        } else {
            Err(LLMClientError::UnSupportedModel)
        }
    }
}
