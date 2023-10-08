use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct AnswerModel {
    /// The name of this model according to tiktoken
    pub tokenizer: &'static str,

    /// The name of this model for use in the llm gateway
    pub model_name: &'static str,

    /// The number of tokens reserved for the answer
    pub answer_tokens: usize,

    /// The number of tokens reserved for the prompt
    pub prompt_tokens_limit: usize,

    /// The number of tokens reserved for history
    pub history_tokens_limit: usize,
}

// GPT-3.5-16k Turbo has 16,385 tokens
pub const GPT_3_5_TURBO_16K: AnswerModel = AnswerModel {
    tokenizer: "gpt-3.5-turbo-16k-0613",
    model_name: "gpt-3.5-turbo-16k-0613",
    answer_tokens: 1024 * 2,
    prompt_tokens_limit: 2500 * 2,
    history_tokens_limit: 2048 * 2,
};

// GPT-4 has 8,192 tokens
pub const GPT_4: AnswerModel = AnswerModel {
    tokenizer: "gpt-4-0613",
    model_name: "gpt-4-0613",
    answer_tokens: 1024,
    prompt_tokens_limit: 2500,
    history_tokens_limit: 2048,
};

// GPT4-32k has 32,769 tokens
pub const GPT_4_32K: AnswerModel = AnswerModel {
    tokenizer: "gpt-4-32k-0613",
    model_name: "gpt-4-32k-0613",
    answer_tokens: 1024 * 4,
    prompt_tokens_limit: 2500 * 4,
    history_tokens_limit: 2048 * 4,
};

impl FromStr for AnswerModel {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[allow(clippy::wildcard_in_or_patterns)]
        match s {
            "gpt-4" => Ok(GPT_4),
            "gpt-4-32k" => Ok(GPT_4_32K),
            "gpt-3.5-turbo-16k" | _ => Ok(GPT_3_5_TURBO_16K),
        }
    }
}

impl<'de> serde::Deserialize<'de> for AnswerModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<AnswerModel>()
            .map_err(|_| serde::de::Error::custom("failed to deserialize"))
    }
}
