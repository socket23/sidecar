use llm_client::{
    clients::{
        anthropic::AnthropicClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMClientRole, LLMType},
    },
    provider::{AnthropicAPIKey, LLMProviderAPIKeys},
};

#[tokio::main]
async fn main() {
    let anthropic_api_key = "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".to_owned();
    let anthropic_client = AnthropicClient::new();
    let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key));
    let system_prompt = r#"You are an intelligent code autocomplete model trained to generate code completions from the cursor position. Given a code snippet with a cursor position marked by <<CURSOR>>, your task is to generate the code that should appear at the <<CURSOR>> to complete the code logically.

To generate the code completion, follow these guidelines:
1. Analyze the code before and after the cursor position to understand the context and intent of the code.
2. If provided, utilize the relevant code snippets from other locations in the codebase to inform your completion. 
3. Generate code that logically continues from the cursor position, maintaining the existing code structure and style.
4. Avoid introducing extra whitespace unless necessary for the code completion.
5. Output only the completed code, without any additional explanations or comments.
6. The code you generate will be inserted at the <<CURSOR>> location, so be mindful to write code that logically follows from the <<CURSOR>> location.
7. You have to always start your reply with <code_inserted> as show in the interactions with the user.
8. You should stop generating code and end with </code_inserted> when you have logically completed the code block you are supposed to autocomplete.
9. Use the same indentation for the generated code as the position of the <<CURSOR>> location. Use spaces if spaces are used; use tabs if tabs are used.
            
Remember, your goal is to provide the most appropriate and efficient code completion based on the given context and the location of the cursor. Use your programming knowledge and the provided examples to generate high-quality code completions that meet the requirements of the task."#;
    let fim_request = r#"<prompt>
<prefix>

use async_trait::async_trait;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

use crate::provider::{LLMProvider, LLMProviderAPIKeys};

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum LLMType {
    Mixtral,
    MistralInstruct,
    Gpt4,
    GPT3_5_16k,
    Gpt4_32k,
    Gpt4Turbo,
    DeepSeekCoder1_3BInstruct,
    DeepSeekCoder33BInstruct,
    DeepSeekCoder6BInstruct,
    CodeLLama70BInstruct,
    CodeLlama13BInstruct,
    CodeLlama7BInstruct,
    ClaudeOpus,
    ClaudeSonnet,
    ClaudeHaiku,
    PPLXSonnetSmall,
    Custom(String),
}

impl Serialize for LLMType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            LLMType::Custom(s) => serializer.serialize_str(s),
            _ => serializer.serialize_str(&format!("{:?}", self)),
        }
    }
}

impl<'de> Deserialize<'de> for LLMType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LLMTypeVisitor;

        impl<'de> Visitor<'de> for LLMTypeVisitor {
            type Value = LLMType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing an LLMType")
            }

            fn visit_str<E>(self, value: &str) -> Result<LLMType, E>
            where
                E: de::Error,
            {
                match value {
                    "Mixtral" => Ok(LLMType::Mixtral),
                    "MistralInstruct" => Ok(LLMType::MistralInstruct),
                    "Gpt4" => Ok(LLMType::Gpt4),
                    "GPT3_5_16k" => Ok(LLMType::GPT3_5_16k),
                    "Gpt4_32k" => Ok(LLMType::Gpt4_32k),
                    "Gpt4Turbo" => Ok(LLMType::Gpt4Turbo),
                    "DeepSeekCoder1.3BInstruct" => Ok(LLMType::DeepSeekCoder1_3BInstruct),
                    "DeepSeekCoder6BInstruct" => Ok(LLMType::DeepSeekCoder6BInstruct),
                    "CodeLLama70BInstruct" => Ok(LLMType::CodeLLama70BInstruct),
                    "CodeLlama13BInstruct" => Ok(LLMType::CodeLlama13BInstruct),
                    "CodeLlama7BInstruct" => Ok(LLMType::CodeLlama7BInstruct),
                    "DeepSeekCoder33BInstruct" => Ok(LLMType::DeepSeekCoder33BInstruct),
                    "ClaudeOpus" => Ok(LLMType::ClaudeOpus),
                    "ClaudeSonnet" => Ok(LLMType::ClaudeSonnet),
                    "ClaudeHaiku" => Ok(LLMType::ClaudeHaiku),
                    "PPLXSonnetSmall" => Ok(LLMType::PPLXSonnetSmall),
                    _ => Ok(LLMType::Custom(value.to_string())),
                }
            }
        }

        deserializer.deserialize_string(LLMTypeVisitor)
    }
}

impl LLMType {
    pub fn is_openai(&self) -> bool {
        matches!(
            self,
            LLMType::Gpt4 | LLMType::GPT3_5_16k | LLMType::Gpt4_32k | LLMType::Gpt4Turbo
        )
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, LLMType::Custom(_))
    }

</prefix>
<insertion_point>
    // check if the model is codellama<<CURSOR>>
</insertion_point>
<suffix>

    pub fn is_anthropic(&self) -> bool {
        matches!(
            self,
            LLMType::ClaudeOpus | LLMType::ClaudeSonnet | LLMType::ClaudeHaiku
        )
    }

    pub fn is_deepseek(&self) -> bool {
        matches!(
            self,
            LLMType::DeepSeekCoder1_3BInstruct
                | LLMType::DeepSeekCoder6BInstruct
                | LLMType::DeepSeekCoder33BInstruct
        )
    }

    pub fn is_togetherai_model(&self) -> bool {
        matches!(
            self,
            LLMType::CodeLlama13BInstruct
                | LLMType::CodeLlama7BInstruct
                | LLMType::DeepSeekCoder33BInstruct
        )
    }
}

impl fmt::Display for LLMType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMType::Mixtral => write!(f, "Mixtral"),
            LLMType::MistralInstruct => write!(f, "MistralInstruct"),
            LLMType::Gpt4 => write!(f, "Gpt4"),
            LLMType::GPT3_5_16k => write!(f, "GPT3_5_16k"),
            LLMType::Gpt4_32k => write!(f, "Gpt4_32k"),
            LLMType::Gpt4Turbo => write!(f, "Gpt4Turbo"),
            LLMType::DeepSeekCoder1_3BInstruct => write!(f, "DeepSeekCoder1.3BInstruct"),
            LLMType::DeepSeekCoder6BInstruct => write!(f, "DeepSeekCoder6BInstruct"),
            LLMType::CodeLLama70BInstruct => write!(f, "CodeLLama70BInstruct"),
            LLMType::CodeLlama13BInstruct => write!(f, "CodeLlama13BInstruct"),
            LLMType::CodeLlama7BInstruct => write!(f, "CodeLlama7BInstruct"),
            LLMType::DeepSeekCoder33BInstruct => write!(f, "DeepSeekCoder33BInstruct"),
            LLMType::ClaudeOpus => write!(f, "ClaudeOpus"),
            LLMType::ClaudeSonnet => write!(f, "ClaudeSonnet"),
            LLMType::ClaudeHaiku => write!(f, "ClaudeHaiku"),
            LLMType::PPLXSonnetSmall => write!(f, "PPLXSonnetSmall"),
            LLMType::Custom(s) => write!(f, "Custom({})", s),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum LLMClientRole {
    System,
    User,
    Assistant,
    // function calling is weird, its only supported by openai right now
    // and not other LLMs, so we are going to make this work with the formatters
    // and still keep it as it is
    Function,
}

impl LLMClientRole {
    pub fn is_system(&self) -> bool {
        matches!(self, LLMClientRole::System)
    }

    pub fn is_user(&self) -> bool {
        matches!(self, LLMClientRole::User)
    }

    pub fn is_assistant(&self) -> bool {
        matches!(self, LLMClientRole::Assistant)
    }

    pub fn is_function(&self) -> bool {
        matches!(self, LLMClientRole::Function)
    }

    pub fn to_string(&self) -> String {
        match self {
            LLMClientRole::System => "system".to_owned(),
            LLMClientRole::User => "user".to_owned(),
            LLMClientRole::Assistant => "assistant".to_owned(),
            LLMClientRole::Function => "function".to_owned(),
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct LLMClientMessageFunctionCall {
    name: String,
    // arguments are generally given as a JSON string, so we keep it as a string
    // here, validate in the upper handlers for this
    arguments: String,
}

impl LLMClientMessageFunctionCall {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn arguments(&self) -> &str {
        &self.arguments
    }
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct LLMClientMessageFunctionReturn {
    name: String,
    content: String,
}

</suffix>
</prompt>

As a reminder the section in <prompt> where you have to make changes is over here
<reminder>

    pub fn is_custom(&self) -> bool {
        matches!(self, LLMType::Custom(_))
    }

<insertion_point>
    // check if the model is codellama<<CURSOR>>
</insertion_point>

    pub fn is_anthropic(&self) -> bool {
        matches!(
            self,
</reminder>"#
        .to_owned();
    let request = LLMClientCompletionRequest::new(
        LLMType::ClaudeHaiku,
        vec![
            LLMClientMessage::new(LLMClientRole::System, system_prompt.to_owned()),
            LLMClientMessage::new(LLMClientRole::User, fim_request.to_owned()).cache_point(),
            //         LLMClientMessage::new(
            //             LLMClientRole::Assistant,
            //             r#"<code_inserted>
            // }

            // // check if the model is codellama"#
            //                 .to_owned(),
            //         )
            //         .cache_point(),
        ],
        0.1,
        None,
    )
    .set_max_tokens(4096);
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let start_instant = std::time::Instant::now();
    let response = anthropic_client
        .stream_completion(api_key, request, sender)
        .await;
    println!("{:?}", response);
    println!("{}", start_instant.elapsed().as_millis());
    // let client = Client::new();
    // let url = "https://api.anthropic.com/v1/messages";
    // let api_key = "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";

    // let response = client
    //     .post(url)
    //     .header("x-api-key", api_key)
    //     .header("anthropic-version", "2023-06-01")
    //     .header("content-type", "application/json")
    //     .json(&json!({
    //         "model": "claude-3-opus-20240229",
    //         "max_tokens": 1024,
    //         "messages": [
    //             {
    //                 "role": "user",
    //                 "content": "Repeat the following content 5 times"
    //             }
    //         ],
    //         "stream": true
    //     }))
    //     .send()
    //     .await
    //     .expect("to work");

    // if response.status().is_success() {
    //     let body = response.text().await.expect("to work");
    //     println!("Response Body: {}", body);
    // } else {
    //     println!("Request failed with status: {}", response.status());
    // }
}
