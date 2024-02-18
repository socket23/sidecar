use axum::{
    response::{sse, IntoResponse, Sse},
    Extension, Json,
};
use chrono::prelude::*;
use futures::{
    stream::{AbortHandle, Abortable},
    StreamExt,
};
use tracing::{error, info};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
}

impl InlineCompletion {
    pub fn new(insert_text: String, insert_range: Range, delta: Option<String>) -> Self {
        Self {
            insert_text,
            insert_range,
            delta,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineCompletionResponse {
    pub completions: Vec<InlineCompletion>,
    pub prompt: String,
}

impl InlineCompletionResponse {
    pub fn new(completions: Vec<InlineCompletion>, prompt: String) -> Self {
        Self {
            completions,
            prompt,
        }
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
    info!(event_name = "inline_completion", id = &id,);
    info!(mode_config = ?model_config);
    dbg!("inline_completion_start", Local::now());
    let fill_in_middle_state = app.fill_in_middle_state.clone();
    let abort_request = fill_in_middle_state.insert(id.clone());
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
            id: id.to_owned(),
        })
        .map_err(|_e| anyhow::anyhow!("error when generating inline completion"))?;
    dbg!("generating_context_finished", Local::now());
    // dbg!(format!("completion streaming start: {}", id));
    // this is how we can abort the running stream if the client disconnects
    let stream = Abortable::new(completions, abort_request);
    Ok(Sse::new(Box::pin(stream.filter_map(
        |completion| async move {
            // dbg!("completion is coming along");
            dbg!(&completion);
            dbg!(Local::now());
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CancelInlineCompletionRequest {
    id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CancelInlineCompletionResponse {}

impl ApiResponse for CancelInlineCompletionResponse {}

pub async fn cancel_inline_completion(
    Extension(app): Extension<Application>,
    Json(CancelInlineCompletionRequest { id }): Json<CancelInlineCompletionRequest>,
) -> Result<impl IntoResponse> {
    dbg!(format!("completion streaming end: {}", &id));
    let fill_in_middle_state = app.fill_in_middle_state.clone();
    fill_in_middle_state.cancel(&id);
    Ok(Json(CancelInlineCompletionResponse {}))
}
