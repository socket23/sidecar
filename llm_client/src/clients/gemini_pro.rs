use futures::StreamExt;
use std::collections::HashMap;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::provider::{LLMProvider, LLMProviderAPIKeys};

use super::types::{
    LLMClient, LLMClientCompletionRequest, LLMClientCompletionResponse,
    LLMClientCompletionStringRequest, LLMClientError, LLMClientMessage, LLMClientRole, LLMType,
};

pub struct GeminiProClient {
    client: reqwest::Client,
}

impl GeminiProClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn get_api_endpoint(&self, project_id: &str) -> String {
        format!("https://us-central1-aiplatform.googleapis.com/v1/projects/{project_id}/locations/us-central1/publishers/google/models/gemini-1.5-pro-preview-0409:streamGenerateContent?alt=sse").to_owned()
    }

    fn model(&self, model: &LLMType) -> Option<String> {
        match model {
            LLMType::GeminiPro => Some("gemini-pro".to_owned()),
            _ => None,
        }
    }

    fn get_system_message(&self, messages: &[LLMClientMessage]) -> Option<SystemInstruction> {
        messages
            .iter()
            .find(|m| m.role().is_system())
            .map(|m| SystemInstruction {
                role: "MODEL".to_owned(),
                parts: vec![HashMap::from([("text".to_owned(), m.content().to_owned())])],
            })
    }

    fn get_role(&self, role: &LLMClientRole) -> Option<String> {
        match role {
            LLMClientRole::System => Some("model".to_owned()),
            LLMClientRole::User => Some("user".to_owned()),
            LLMClientRole::Assistant => Some("model".to_owned()),
            _ => None,
        }
    }

    fn get_generation_config(&self, request: &LLMClientCompletionRequest) -> GenerationConfig {
        GenerationConfig {
            temperature: request.temperature(),
            // this is the maximum limit of gemini-pro-1.5
            max_output_tokens: 8192,
            candidate_count: 1,
            top_p: None,
            top_k: None,
        }
    }

    fn get_messages(&self, messages: &[LLMClientMessage]) -> Vec<Content> {
        messages
            .iter()
            .filter(|m| !m.role().is_system())
            .filter_map(|m| {
                if let Some(role) = self.get_role(&m.role()) {
                    Some(Content {
                        role,
                        parts: vec![HashMap::from([("text".to_owned(), m.content().to_owned())])],
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn get_api_key(&self, api_key: &LLMProviderAPIKeys) -> Option<String> {
        match api_key {
            LLMProviderAPIKeys::GeminiPro(key) => Some(key.api_key.to_owned()),
            _ => None,
        }
    }

    fn get_api_base(&self, api_key: &LLMProviderAPIKeys) -> Option<String> {
        match api_key {
            LLMProviderAPIKeys::GeminiPro(api_key) => Some(api_key.api_base.to_owned()),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    top_p: Option<f32>,
    top_k: Option<u32>,
    max_output_tokens: u32,
    candidate_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    role: String,
    // the only parts we will be providing is "text": "content"
    parts: Vec<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SystemInstruction {
    role: String,
    // the only parts we will be providing is "text": "content"
    parts: Vec<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiSafetySetting {
    #[serde(rename = "category")]
    category: String,
    #[serde(rename = "threshold")]
    threshold: String,
}

impl GeminiSafetySetting {
    pub fn new(category: String, threshold: String) -> Self {
        Self {
            category,
            threshold,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiProRequestBody {
    contents: Vec<Content>,
    system_instruction: Option<SystemInstruction>,
    generation_config: GenerationConfig,
    safety_settings: Vec<GeminiSafetySetting>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiProResponse {
    candidates: Vec<GeminiProCandidate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiProCandidate {
    content: Content,
    // safety_ratings: Vec<GeminiProSafetyRating>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiProSafetyRating {
    category: String,
    probability: String,
    probability_score: f32,
    severity: String,
    severity_score: f32,
}
#[async_trait]
impl LLMClient for GeminiProClient {
    fn client(&self) -> &LLMProvider {
        &LLMProvider::GeminiPro
    }

    async fn stream_completion(
        &self,
        provider_api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        let model = self.model(request.model());
        if model.is_none() {
            return Err(LLMClientError::UnSupportedModel);
        }
        let model = model.unwrap();
        let system_message = self.get_system_message(request.messages());
        let messages = self.get_messages(request.messages());
        let generation_config = self.get_generation_config(&request);
        let request = GeminiProRequestBody {
            contents: messages,
            system_instruction: system_message,
            generation_config,
            safety_settings: vec![
                GeminiSafetySetting::new(
                    "HARM_CATEGORY_HATE_SPEECH".to_string(),
                    "BLOCK_ONLY_HIGH".to_string(),
                ),
                GeminiSafetySetting::new(
                    "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                    "BLOCK_ONLY_HIGH".to_string(),
                ),
                GeminiSafetySetting::new(
                    "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                    "BLOCK_ONLY_HIGH".to_string(),
                ),
                GeminiSafetySetting::new(
                    "HARM_CATEGORY_HARASSMENT".to_string(),
                    "BLOCK_ONLY_HIGH".to_string(),
                ),
            ],
        };
        println!("{:?}", serde_json::to_string(&request));
        let api_key = self.get_api_key(&provider_api_key);
        let api_base = self.get_api_base(&provider_api_key);
        if api_key.is_none() || api_base.is_none() {
            return Err(LLMClientError::WrongAPIKeyType);
        }
        let api_key = api_key.expect("to be present");
        let api_base = api_base.expect("to be present");
        dbg!(&api_key, &api_base);
        // now we need to send a request to the gemini pro api here
        let response = dbg!(
            self.client
                .post(self.get_api_endpoint(&api_base))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
        )?;

        if !response.status().is_success() {
            return Err(LLMClientError::FailedToGetResponse);
        }

        let mut buffered_string = "".to_owned();
        let mut response_stream = response.bytes_stream().eventsource();
        while let Some(event) = response_stream.next().await {
            println!("{:?}", event);
            if let Ok(event) = event {
                let parsed_event =
                    serde_json::from_slice::<GeminiProResponse>(event.data.as_bytes())?;
                if let Some(text_part) = parsed_event.candidates[0].content.parts[0].get("text") {
                    buffered_string = buffered_string + text_part;
                    sender.send(LLMClientCompletionResponse::new(
                        buffered_string.clone(),
                        Some(text_part.to_owned()),
                        model.to_owned(),
                    ))?;
                }
            }
        }
        Ok(buffered_string)
    }

    async fn completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError> {
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        self.stream_completion(api_key, request, sender).await
    }

    async fn stream_prompt_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError> {
        Err(LLMClientError::GeminiProDoesNotSupportPromptCompletion)
    }
}
