//! OpenAI Compatible Client Implementation
//! 
//! This module provides an implementation of the LLMClient trait for OpenAI-compatible APIs.
//! It supports both the official OpenAI API and other compatible endpoints that follow the
//! same API structure.
//! 
//! # Features
//! 
//! - Chat completion with streaming support
//! - Prompt completion with streaming support
//! - Function calling capability
//! - Custom model support
//! - Comprehensive error handling
//! 
//! # Example
//! 
//! ```no_run
//! use crate::clients::{OpenAICompatibleClient, LLMClient};
//! use crate::provider::{LLMProviderAPIKeys, OpenAICompatibleAPIKey};
//! 
//! let client = OpenAICompatibleClient::new();
//! let messages = vec![LLMClientMessage::user("Hello!".to_owned())];
//! let request = LLMClientCompletionRequest::new(
//!     LLMType::Gpt4,
//!     messages,
//!     0.7,
//!     None,
//! );
//! 
//! let api_key = LLMProviderAPIKeys::OpenAICompatible(OpenAICompatibleAPIKey {
//!     api_key: "your-api-key".to_string(),
//!     api_base: "https://api.openai.com/v1".to_string(),
//! });
//! 
//! let result = client.completion(api_key, request).await?;
//! ```

use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, Choice,
        CreateChatCompletionRequestArgs, CreateCompletionRequestArgs, FunctionCall, Role,
    },
    Client,
};
use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc::UnboundedSender;


use crate::provider::{LLMProvider, LLMProviderAPIKeys};

use super::types::{
    LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse,
    LLMClientCompletionStringRequest, LLMClientError, LLMClientMessage, LLMClientRole, LLMType,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PartialOpenAIResponse {
    choices: Vec<Choice>,
}

#[async_trait]
impl LLMClient for OpenAICompatibleClient {
    fn client(&self) -> &LLMProvider {
        unimplemented!("OpenAI compatible client does not implement client()")
    }

    async fn stream_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        let model = self
            .model(&request.model())
            .ok_or(LLMClientError::UnSupportedModel)?;
        
        let messages = self.messages(&request.messages())?;
        let client = self.generate_openai_client(api_key, request.model())?;

        match client {
            OpenAIClientType::OpenAIClient(client) => {
                let request = CreateChatCompletionRequestArgs::default()
                    .model(model)
                    .messages(messages)
                    .temperature(request.temperature())
                    .max_tokens(request.get_max_tokens())
                    .stream(true)
                    .build()?;

                let mut stream = client.chat().create_stream(request).await?;
                let mut completion_content = String::new();
                
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(response) => {
                            if let Some(choice) = response.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    completion_content.push_str(content);
                                    sender.send(LLMClientCompletionResponse::new(
                                        completion_content.clone(),
                                        Some(content.to_string()),
                                        model.clone(),
                                    )).map_err(|_| LLMClientError::TokioMpscSendError)?;
                                }
                            }
                        }
                        Err(err) => return Err(LLMClientError::OpenAPIError(err)),
                    }
                }

                Ok(completion_content)
            }
        }
    }

    async fn completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let content = self.stream_completion(api_key, request, tx).await?;
        while rx.recv().await.is_some() {
            // Drain the channel
        }
        Ok(content)
    }

    async fn stream_prompt_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        let model = self
            .model(&request.model())
            .ok_or(LLMClientError::UnSupportedModel)?;
        
        let client = self.generate_completion_openai_client(api_key, request.model())?;
        
        let request = CreateCompletionRequestArgs::default()
            .model(model.clone())
            .prompt(request.prompt())
            .temperature(request.temperature())
            .max_tokens(request.get_max_tokens())
            .stream(true)
            .build()?;

        let mut stream = client.completions().create_stream(request).await?;
        let mut completion_content = String::new();
        
        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        if let Some(content) = &choice.text {
                            completion_content.push_str(content);
                            sender.send(LLMClientCompletionResponse::new(
                                completion_content.clone(),
                                Some(content.to_string()),
                                model.clone(),
                            )).map_err(|_| LLMClientError::TokioMpscSendError)?;
                        }
                    }
                }
                Err(err) => return Err(LLMClientError::OpenAPIError(err)),
            }
        }

        Ok(completion_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{AnthropicAPIKey, OpenAICompatibleAPIKey};
    use tokio_test;

    #[test]
    fn test_model_mapping() {
        let client = OpenAICompatibleClient::new();
        
        // Test known models
        assert_eq!(
            client.model(&LLMType::GPT3_5_16k),
            Some("gpt-3.5-turbo-16k-0613".to_owned())
        );
        assert_eq!(
            client.model(&LLMType::Gpt4),
            Some("gpt-4-0613".to_owned())
        );
        
        // Test custom model
        let custom_model = "custom-model-name";
        assert_eq!(
            client.model(&LLMType::Custom(custom_model.to_owned())),
            Some(custom_model.to_owned())
        );
        
        // Test unsupported model
        assert_eq!(client.model(&LLMType::Mixtral), None);
    }

    #[test]
    fn test_message_conversion() {
        let client = OpenAICompatibleClient::new();
        
        let messages = vec![
            LLMClientMessage::system("System message".to_owned()),
            LLMClientMessage::user("User message".to_owned()),
            LLMClientMessage::assistant("Assistant message".to_owned()),
        ];
        
        let result = client.messages(&messages);
        assert!(result.is_ok());
        
        let converted_messages = result.unwrap();
        assert_eq!(converted_messages.len(), 3);
    }

    #[test]
    fn test_invalid_api_key_type() {
        let client = OpenAICompatibleClient::new();
        let result = client.generate_openai_client(
            LLMProviderAPIKeys::Anthropic(AnthropicAPIKey {
                api_key: "test".to_string(),
            }),
            &LLMType::Gpt4,
        );
        assert!(matches!(result, Err(LLMClientError::WrongAPIKeyType)));
    }

    #[test]
    fn test_function_call_conversion() {
        let client = OpenAICompatibleClient::new();
        let function_message = LLMClientMessage::function_call(
            "test_function".to_owned(),
            r#"{"arg": "value"}"#.to_owned(),
        );
        
        let messages = vec![function_message];
        let result = client.messages(&messages);
        assert!(result.is_ok());
        
        let converted_messages = result.unwrap();
        assert_eq!(converted_messages.len(), 1);
    }

    #[test]
    fn test_function_without_call() {
        let client = OpenAICompatibleClient::new();
        let function_message = LLMClientMessage::function("test response".to_owned());
        
        let messages = vec![function_message];
        let result = client.messages(&messages);
        assert!(matches!(result, Err(LLMClientError::FunctionCallNotPresent)));
    }

    #[test]
    fn test_unsupported_model_validation() {
        let client = OpenAICompatibleClient::new();
        let messages = vec![LLMClientMessage::user("Test message".to_owned())];
        let request = LLMClientCompletionRequest::new(
            LLMType::Mixtral,
            messages,
            0.7,
            None,
        );
        
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let result = tokio_test::block_on(client.stream_completion(
            LLMProviderAPIKeys::OpenAICompatible(OpenAICompatibleAPIKey {
                api_key: "test".to_string(),
                api_base: "test".to_string(),
            }),
            request,
            tx,
        ));
        
        assert!(matches!(result, Err(LLMClientError::UnSupportedModel)));
    }

    #[test]
    fn test_supported_model_validation() {
        let client = OpenAICompatibleClient::new();
        let messages = vec![LLMClientMessage::user("Test message".to_owned())];
        let request = LLMClientCompletionRequest::new(
            LLMType::Gpt4,
            messages,
            0.7,
            None,
        );
        
        let model = client.model(&request.model());
        assert!(model.is_some());
        assert_eq!(model.unwrap(), "gpt-4-0613");
    }
}



enum OpenAIClientType {
    OpenAIClient(Client<OpenAIConfig>),
}

/// A client implementation for OpenAI-compatible APIs.
/// This includes both the official OpenAI API and other compatible endpoints
/// that follow the same API structure.
pub struct OpenAICompatibleClient {
    // Add any configuration fields if needed in the future
}

impl OpenAICompatibleClient {
    /// Creates a new instance of OpenAICompatibleClient
    pub fn new() -> Self {
        Self {}
    }

    /// Maps an LLMType to its corresponding model string for OpenAI-compatible APIs
    pub fn model(&self, model: &LLMType) -> Option<String> {
        match model {
            LLMType::GPT3_5_16k => Some("gpt-3.5-turbo-16k-0613".to_owned()),
            LLMType::Gpt4 => Some("gpt-4-0613".to_owned()),
            LLMType::Gpt4Turbo => Some("gpt-4-1106-preview".to_owned()),
            LLMType::Gpt4_32k => Some("gpt-4-32k-0613".to_owned()),
            LLMType::DeepSeekCoder33BInstruct => Some("deepseek-coder-33b".to_owned()),
            LLMType::DeepSeekCoder6BInstruct => Some("deepseek-coder-6b".to_owned()),
            LLMType::CodeLlama13BInstruct => Some("codellama-13b".to_owned()),
            LLMType::Llama3_1_8bInstruct => Some("llama-3.1-8b-instant".to_owned()),
            LLMType::Llama3_1_70bInstruct => Some("llama-3.1-70b-versatile".to_owned()),
            LLMType::Custom(name) => Some(name.to_owned()),
            _ => None,
        }
    }

    /// Converts LLMClientMessage to OpenAI's ChatCompletionRequestMessage format
    pub fn messages(
        &self,
        messages: &[LLMClientMessage],
    ) -> Result<Vec<ChatCompletionRequestMessage>, LLMClientError> {
        messages
            .iter()
            .map(|message| {
                let role = message.role();
                match role {
                    LLMClientRole::User => ChatCompletionRequestUserMessageArgs::default()
                        .role(Role::User)
                        .content(message.content().to_owned())
                        .build()
                        .map(|message| ChatCompletionRequestMessage::User(message))
                        .map_err(|e| LLMClientError::OpenAPIError(e)),
                    LLMClientRole::System => ChatCompletionRequestSystemMessageArgs::default()
                        .role(Role::System)
                        .content(message.content().to_owned())
                        .build()
                        .map(|message| ChatCompletionRequestMessage::System(message))
                        .map_err(|e| LLMClientError::OpenAPIError(e)),
                    // TODO(skcd): This might be wrong, but for now its okay as we
                    // do not use these branches at all
                    LLMClientRole::Assistant => match message.get_function_call() {
                        Some(function_call) => ChatCompletionRequestAssistantMessageArgs::default()
                            .role(Role::Function)
                            .function_call(FunctionCall {
                                name: function_call.name().to_owned(),
                                arguments: function_call.arguments().to_owned(),
                            })
                            .build()
                            .map(|message| ChatCompletionRequestMessage::Assistant(message))
                            .map_err(|e| LLMClientError::OpenAPIError(e)),
                        None => ChatCompletionRequestAssistantMessageArgs::default()
                            .role(Role::Assistant)
                            .content(message.content().to_owned())
                            .build()
                            .map(|message| ChatCompletionRequestMessage::Assistant(message))
                            .map_err(|e| LLMClientError::OpenAPIError(e)),
                    },
                    LLMClientRole::Function => match message.get_function_call() {
                        Some(function_call) => ChatCompletionRequestAssistantMessageArgs::default()
                            .role(Role::Function)
                            .content(message.content().to_owned())
                            .function_call(FunctionCall {
                                name: function_call.name().to_owned(),
                                arguments: function_call.arguments().to_owned(),
                            })
                            .build()
                            .map(|message| ChatCompletionRequestMessage::Assistant(message))
                            .map_err(|e| LLMClientError::OpenAPIError(e)),
                        None => Err(LLMClientError::FunctionCallNotPresent),
                    },
                }
            })
            .collect::<Vec<_>>();
        formatted_messages
            .into_iter()
            .collect::<Result<Vec<ChatCompletionRequestMessage>, LLMClientError>>()
    }

    /// Generates an OpenAI client with the provided configuration
    fn generate_openai_client(
        &self,
        api_key: LLMProviderAPIKeys,
        _llm_model: &LLMType,
    ) -> Result<OpenAIClientType, LLMClientError> {
        match api_key {
            LLMProviderAPIKeys::OpenAICompatible(openai_compatible) => {
                let config = OpenAIConfig::new()
                    .with_api_key(openai_compatible.api_key)
                    .with_api_base(openai_compatible.api_base);
                Ok(OpenAIClientType::OpenAIClient(Client::with_config(config)))
            }
            _ => Err(LLMClientError::WrongAPIKeyType),
        }
    }

    /// Generates an OpenAI client specifically for completion requests
    fn generate_completion_openai_client(
        &self,
        api_key: LLMProviderAPIKeys,
        _llm_model: &LLMType,
    ) -> Result<Client<OpenAIConfig>, LLMClientError> {
        match api_key {
            LLMProviderAPIKeys::OpenAICompatible(openai_compatible) => {
                let config = OpenAIConfig::new()
                    .with_api_key(openai_compatible.api_key)
                    .with_api_base(openai_compatible.api_base);
                Ok(Client::with_config(config))
            }
            _ => Err(LLMClientError::WrongAPIKeyType),
        }
    }
}

