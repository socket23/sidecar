use std::path::PathBuf;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    config::LLMBrokerConfiguration,
    provider::{LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};
use sidecar::agentic::symbol::identifier::LLMProperties;

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    let gpt4o_config = LLMProperties::new(
        LLMType::Gpt4O,
        LLMProvider::OpenAI,
        LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
            "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".to_owned(),
        )),
    );
    let llm_client = LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
        .await
        .expect("to work");
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = llm_client
        .stream_completion(
            gpt4o_config.api_key().clone(),
            LLMClientCompletionRequest::new(
                gpt4o_config.llm().clone(),
                vec![
                    LLMClientMessage::system("you are an expert at saying hi".to_owned()),
                    LLMClientMessage::user("say hi to me".to_owned()),
                ],
                0.2,
                None,
            ),
            gpt4o_config.provider().clone(),
            vec![].into_iter().collect(),
            sender,
        )
        .await;
    println!("{:?}", response);
}
