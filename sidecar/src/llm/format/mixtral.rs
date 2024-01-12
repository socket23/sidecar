use crate::llm::clients::types::LLMClientMessage;

use super::types::{LLMFormatting, TokenizerConfig, TokenizerError};

pub struct MixtralInstructFormatting {
    tokenizer_config: TokenizerConfig,
}

impl MixtralInstructFormatting {
    pub fn new() -> Result<Self, TokenizerError> {
        let config = include_str!("tokenizer_config/mistral.json");
        let tokenizer_config = serde_json::from_str::<TokenizerConfig>(config)?;
        Ok(Self { tokenizer_config })
    }
}

impl LLMFormatting for MixtralInstructFormatting {
    fn to_prompt(&self, messages: Vec<LLMClientMessage>) -> String {
        // we want to convert the message to mistral format
        // persent here: https://huggingface.co/mistralai/Mixtral-8x7B-Instruct-v0.1/blob/main/tokenizer_config.json
        // {{ bos_token }}
        // {% for message in messages %}
        // {% if (message['role'] == 'user') != (loop.index0 % 2 == 0) %}
        // {{ raise_exception('Conversation roles must alternate user/assistant/user/assistant/...') }}
        // {% endif %}
        // {% if message['role'] == 'user' %}
        // {{ '[INST] ' + message['content'] + ' [/INST]' }}
        // {% elif message['role'] == 'assistant' %}
        // {{ message['content'] + eos_token}}
        // {% else %}
        // {{ raise_exception('Only user and assistant roles are supported!') }}
        // {% endif %}{% endfor %}
        // First the messages have to be alternating, if that's not enforced then we run into problems
        // but since thats the case, we can do something better, which is to to just send consecutive messages
        // from human and assistant together
        let formatted_message = messages
            .into_iter()
            .skip_while(|message| message.role().is_assistant())
            .map(|message| {
                let content = message.content();
                let eos_token = self.tokenizer_config.eos_token();
                if message.role().is_system() || message.role().is_user() {
                    format!("[INST] {content} [/INST]")
                } else {
                    format!("{content}{eos_token}")
                }
            })
            .collect::<Vec<_>>()
            .join("");
        format!("<s>{formatted_message}")
    }
}

#[cfg(test)]
mod tests {
    use crate::llm::clients::types::LLMClientMessage;

    use super::LLMFormatting;
    use super::MixtralInstructFormatting;

    #[test]
    fn test_formatting_works() {
        let messages = vec![
            LLMClientMessage::user("user_msg1".to_owned()),
            LLMClientMessage::assistant("assistant_msg1".to_owned()),
        ];
        let mistral_formatting = MistralInstructFormatting::new().unwrap();
        assert_eq!(
            mistral_formatting.to_prompt(messages),
            "<s>[INST] user_msg1 [/INST]assistant_msg1</s>",
        );
    }
}
