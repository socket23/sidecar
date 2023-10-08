/// We define all the helper stuff required here for the LLM to be able to do
/// things.
use async_openai::config::AzureConfig;
use async_openai::config::Config;
use async_openai::types::ChatCompletionFunctionsArgs;
use async_openai::types::ChatCompletionRequestMessageArgs;
use async_openai::types::CreateChatCompletionRequest;
use async_openai::types::CreateChatCompletionRequestArgs;
use async_openai::types::FunctionCall;
use async_openai::Client;
use color_eyre::owo_colors::colors::css::Azure;
use futures::StreamExt;
use tracing::warn;

use std::sync::Arc;

use self::llm::Function;
use self::llm::Message;

pub mod llm {
    use std::collections::HashMap;

    #[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct FunctionCall {
        pub name: String,
        pub arguments: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Function {
        pub name: String,
        pub description: String,
        pub parameters: Parameters,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Parameters {
        #[serde(rename = "type")]
        pub _type: String,
        pub properties: HashMap<String, Parameter>,
        pub required: Vec<String>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    pub enum Role {
        User,
        System,
        Assistant,
        Function,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Parameter {
        #[serde(rename = "type")]
        pub _type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub items: Option<Box<Parameter>>,
    }
    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
    #[serde(untagged)]
    pub enum Message {
        FunctionReturn {
            role: Role,
            name: String,
            content: String,
        },
        FunctionCall {
            role: Role,
            function_call: FunctionCall,
            content: (),
        },
        // NB: This has to be the last variant as this enum is marked `#[serde(untagged)]`, so
        // deserialization will always try this variant last. Otherwise, it is possible to
        // accidentally deserialize a `FunctionReturn` value as `PlainText`.
        PlainText {
            role: Role,
            content: String,
        },
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
    pub struct Messages {
        pub messages: Vec<Message>,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
    pub struct Functions {
        pub functions: Vec<Function>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    pub struct Request {
        pub messages: Messages,
        pub functions: Option<Functions>,
        pub provider: Provider,
        pub max_tokens: Option<u32>,
        pub temperature: Option<f32>,
        pub presence_penalty: Option<f32>,
        pub frequency_penalty: Option<f32>,
        pub model: Option<String>,
        #[serde(default)]
        pub extra_stop_sequences: Vec<String>,
        pub session_reference_id: Option<String>,
    }

    #[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Provider {
        OpenAi,
    }

    #[derive(thiserror::Error, Debug, serde::Deserialize, serde::Serialize)]
    pub enum Error {
        #[error("bad OpenAI request")]
        BadOpenAiRequest,

        #[error("incorrect configuration")]
        BadConfiguration,
    }

    #[derive(Debug, Clone)]
    pub enum OpenAIModel {
        GPT4,
        GPT4_32k,
        GPT3_5_16k,
    }

    impl OpenAIModel {
        pub fn model_name(&self) -> String {
            match self {
                OpenAIModel::GPT3_5_16k => "gpt-3.5-turbo-16k".to_owned(),
                OpenAIModel::GPT4 => "gpt-4".to_owned(),
                OpenAIModel::GPT4_32k => "gpt-4-32k".to_owned(),
            }
        }
    }

    pub type Result = std::result::Result<String, Error>;
}

impl llm::Message {
    pub fn system(content: &str) -> Self {
        llm::Message::PlainText {
            role: llm::Role::System,
            content: content.to_owned(),
        }
    }
}

impl From<&llm::Role> for async_openai::types::Role {
    fn from(role: &llm::Role) -> Self {
        match role {
            llm::Role::User => async_openai::types::Role::User,
            llm::Role::System => async_openai::types::Role::System,
            llm::Role::Assistant => async_openai::types::Role::Assistant,
            llm::Role::Function => async_openai::types::Role::Function,
        }
    }
}

pub struct LlmClient {
    gpt4_client: Client<AzureConfig>,
    gpt432k_client: Client<AzureConfig>,
    gpt3_5_client: Client<AzureConfig>,
}

impl LlmClient {
    pub fn codestory_infra() -> LlmClient {
        let api_base = "https://codestory-gpt4.openai.azure.com".to_owned();
        let api_key = "89ca8a49a33344c9b794b3dabcbbc5d0".to_owned();
        let api_version = "2023-08-01-preview".to_owned();
        let azure_config = AzureConfig::new()
            .with_api_base(api_base)
            .with_api_key(api_key)
            .with_api_version(api_version)
            .with_deployment_id("gpt4-access".to_owned());
        let gpt4_config = azure_config.clone();
        let gpt4_32k_config = azure_config
            .clone()
            .with_deployment_id("gpt4-32k-access".to_owned());
        let gpt3_5_config = azure_config.with_deployment_id("gpt35-turbo-access".to_owned());
        Self {
            gpt4_client: Client::with_config(gpt4_config),
            gpt432k_client: Client::with_config(gpt4_32k_config),
            gpt3_5_client: Client::with_config(gpt3_5_config),
        }
    }

    pub async fn stream_response(
        &self,
        model: llm::OpenAIModel,
        messages: Vec<llm::Message>,
        functions: Vec<llm::Function>,
        temperature: f32,
    ) -> anyhow::Result<String> {
        let client = match model {
            llm::OpenAIModel::GPT4 => &self.gpt4_client,
            llm::OpenAIModel::GPT4_32k => &self.gpt432k_client,
            llm::OpenAIModel::GPT3_5_16k => &self.gpt3_5_client,
        };
        let request = self.create_request(messages, functions, temperature);

        const TOTAL_CHAT_RETRIES: usize = 5;

        'retry_loop: for _ in 0..TOTAL_CHAT_RETRIES {
            let mut buf = String::new();
            let stream = client.chat().create_stream(request.clone()).await;
            if stream.is_err() {
                continue 'retry_loop;
            }
            let unwrap_stream = stream.expect("is_err check above to work");
            tokio::pin!(unwrap_stream);

            loop {
                match unwrap_stream.next().await {
                    None => break,
                    Some(Ok(s)) => {
                        buf += &s
                            .choices
                            .get(0)
                            .map(|choice| choice.delta.content.clone())
                            .flatten()
                            .unwrap_or("".to_owned())
                    }
                    Some(Err(e)) => {
                        warn!(?e, "openai stream error, retrying");
                        continue 'retry_loop;
                    }
                }
            }

            return Ok(buf);
        }
        Err(anyhow::anyhow!(
            "failed to get response from openai".to_owned()
        ))
    }

    fn create_request(
        &self,
        messages: Vec<llm::Message>,
        functions: Vec<llm::Function>,
        temperature: f32,
    ) -> CreateChatCompletionRequest {
        let request_messages: Vec<_> = messages
            .into_iter()
            .map(|message| match message {
                llm::Message::PlainText { role, content } => {
                    ChatCompletionRequestMessageArgs::default()
                        .role::<async_openai::types::Role>((&role).into())
                        .content(content)
                        .build()
                        .unwrap()
                }
                llm::Message::FunctionCall {
                    role,
                    function_call,
                    content,
                } => ChatCompletionRequestMessageArgs::default()
                    .role::<async_openai::types::Role>((&role).into())
                    .function_call(FunctionCall {
                        name: function_call.name,
                        arguments: function_call.arguments,
                    })
                    .build()
                    .unwrap(),
                llm::Message::FunctionReturn {
                    role,
                    name,
                    content,
                } => ChatCompletionRequestMessageArgs::default()
                    .role::<async_openai::types::Role>((&role).into())
                    .content(content)
                    .build()
                    .unwrap(),
            })
            .collect();
        let function_calling: Vec<_> = functions
            .into_iter()
            .map(|function| {
                ChatCompletionFunctionsArgs::default()
                    .name(function.name)
                    .description(function.description)
                    .parameters(
                        serde_json::to_value(function.parameters)
                            .expect("serde_json::unable_to_convert"),
                    )
                    .build()
                    .unwrap()
            })
            .collect();
        CreateChatCompletionRequestArgs::default()
            .messages(request_messages)
            .functions(function_calling)
            .temperature(temperature)
            .stream(true)
            .build()
            .expect("chat_completion_request_builder to not fail")
    }
}
