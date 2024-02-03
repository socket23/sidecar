use axum::{
    response::{sse, IntoResponse, Sse},
    Extension, Json,
};
use futures::StreamExt;
use tracing::error;

use crate::{
    application::application::Application,
    chunking::text_document::{Position, Range},
    in_line_agent::types::InLineAgent,
    inline_completion::types::{FillInMiddleCompletionAgent, InLineCompletionError},
};

use super::{
    model_selection::LLMClientConfig,
    types::{ApiResponse, Result},
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct InlineCompletionRequest {
    pub filepath: String,
    pub language: String,
    pub text: String,
    pub position: Position,
    pub indentation: Option<String>,
    pub model_config: LLMClientConfig,
    pub id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineCompletion {
    pub insert_text: String,
    pub insert_range: Range,
}

impl InlineCompletion {
    pub fn new(insert_text: String, insert_range: Range) -> Self {
        Self {
            insert_text,
            insert_range,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineCompletionResponse {
    pub completions: Vec<InlineCompletion>,
}

impl InlineCompletionResponse {
    pub fn new(completions: Vec<InlineCompletion>) -> Self {
        Self { completions }
    }
}

impl ApiResponse for InlineCompletionResponse {}

pub async fn inline_completion(
    Extension(app): Extension<Application>,
    Json(InlineCompletionRequest {
        filepath,
        language,
        text,
        position,
        indentation,
        model_config,
        id,
    }): Json<InlineCompletionRequest>,
) -> Result<impl IntoResponse> {
    let fill_in_middle_agent = FillInMiddleCompletionAgent::new(
        app.llm_broker.clone(),
        app.llm_tokenizer.clone(),
        app.answer_models.clone(),
        app.fill_in_middle_broker.clone(),
        app.editor_parsing.clone(),
    );
    let completions = fill_in_middle_agent
        .completion(InlineCompletionRequest {
            filepath,
            language,
            text,
            position,
            indentation,
            model_config,
            id,
        })
        .map_err(|_e| anyhow::anyhow!("error when generating inline completion"))?;
    Ok(Sse::new(Box::pin(completions.filter_map(
        |completion| async move {
            match completion {
                Ok(completion) => Some(
                    sse::Event::default()
                        .json_data(serde_json::to_string(&completion).expect("serde to work")),
                ),
                _ => None,
            }
        },
    ))))
}
