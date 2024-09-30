//! Contains the helper functions over here for the plan generation

use std::sync::Arc;

use super::types::Result;
use axum::response::{sse, Sse};
use futures::StreamExt;
use llm_client::clients::types::LLMClientCompletionResponse;
use serde_json::json;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    agent::types::{AgentAnswerStreamEvent, ConversationMessage},
    agentic::{
        symbol::events::{
            input::SymbolEventRequestId, message_event::SymbolEventMessageProperties,
        },
        tool::plan::service::PlanService,
    },
    application::config::configuration::Configuration,
    user_context::types::UserContext,
};

/// Create the plan using the context present over here
pub async fn create_plan(
    user_query: String,
    user_context: UserContext,
    editor_url: String,
    plan_id: uuid::Uuid,
    plan_storage_path: String,
    plan_service: PlanService,
    // we can send events using this
    agent_sender: UnboundedSender<anyhow::Result<ConversationMessage>>,
) {
    let _ = agent_sender.send(Ok(ConversationMessage::answer_update(
        plan_id.clone(),
        AgentAnswerStreamEvent::LLMAnswer(LLMClientCompletionResponse::new(
            "Generating plan".to_owned(),
            Some("Generating plan".to_owned()),
            "Custom".to_owned(),
        )),
    )));
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let plan_id_str = plan_id.to_string();
    let message_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new(plan_id_str.to_owned(), plan_id_str.to_owned()),
        sender,
        editor_url,
        cancellation_token,
    );

    let plan = plan_service
        .create_plan(
            plan_id_str,
            user_query,
            user_context,
            plan_storage_path.to_owned(),
            message_properties,
        )
        .await;

    match plan {
        Ok(plan) => {
            // send over a response that we are done generating the plan
            let final_answer = format!(
                r#"finished generating plan at [location]({})
plan_information:
{}"#,
                &plan_storage_path,
                plan.to_debug_message(),
            );
            let _ = plan_service.save_plan(&plan, &plan_storage_path).await;
            let _ = agent_sender.send(Ok(ConversationMessage::answer_update(
                plan_id,
                AgentAnswerStreamEvent::LLMAnswer(LLMClientCompletionResponse::new(
                    final_answer.to_owned(),
                    Some(final_answer.to_owned()),
                    "Custom".to_owned(),
                )),
            )));
        }
        Err(e) => {
            eprintln!("Failed to generate plan: {:?}", e);
            let final_answer = "Failed to generate plan".to_owned();
            let _ = agent_sender.send(Ok(ConversationMessage::answer_update(
                plan_id,
                AgentAnswerStreamEvent::LLMAnswer(LLMClientCompletionResponse::new(
                    final_answer.to_owned(),
                    Some(final_answer.to_owned()),
                    "Custom".to_owned(),
                )),
            )));
        }
    }
    // drop the sender over here
    drop(agent_sender);
}

pub async fn handle_create_plan(
    user_query: String,
    user_context: UserContext,
    editor_url: String,
    plan_id: uuid::Uuid,
    plan_storage_path: String,
    plan_service: PlanService,
) -> Result<
    Sse<std::pin::Pin<Box<dyn tokio_stream::Stream<Item = anyhow::Result<sse::Event>> + Send>>>,
> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    // we let the plan creation happen in the background
    let _ = tokio::spawn(async move {
        create_plan(
            user_query,
            user_context,
            editor_url,
            plan_id,
            plan_storage_path,
            plan_service,
            sender,
        )
        .await;
    });
    let conversation_message_stream =
        tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    // TODO(skcd): Re-introduce this again when we have a better way to manage
    // server side events on the client side
    let init_stream = futures::stream::once(async move {
        Ok(sse::Event::default()
            .json_data(json!({
                "session_id": plan_id.to_owned(),
            }))
            // This should never happen, so we force an unwrap.
            .expect("failed to serialize initialization object"))
    });

    // // We know the stream is unwind safe as it doesn't use synchronization primitives like locks.
    let answer_stream = conversation_message_stream.map(
        |conversation_message: anyhow::Result<ConversationMessage>| {
            if let Err(e) = &conversation_message {
                tracing::error!("error in conversation message stream: {}", e);
            }
            sse::Event::default()
                .json_data(conversation_message.expect("should not fail deserialization"))
                .map_err(anyhow::Error::new)
        },
    );

    // TODO(skcd): Re-introduce this again when we have a better way to manage
    // server side events on the client side
    let done_stream = futures::stream::once(async move {
        Ok(sse::Event::default()
            .json_data(json!(
                {"done": "[CODESTORY_DONE]".to_owned(),
                "session_id": plan_id.to_owned(),
            }))
            .expect("failed to send done object"))
    });

    let stream = init_stream.chain(answer_stream).chain(done_stream);

    Ok(Sse::new(Box::pin(stream)))
}

pub async fn check_plan_storage_path(config: Arc<Configuration>, plan_id: String) -> String {
    let mut plan_path = config.index_dir.clone();
    plan_path = plan_path.join("plans");
    // check if the plan_storage_path_exists
    if tokio::fs::metadata(&plan_path).await.is_err() {
        tokio::fs::create_dir(&plan_path)
            .await
            .expect("directory creation to not fail");
    }
    plan_path = plan_path.join(plan_id);
    plan_path
        .to_str()
        .expect("path conversion to work on all platforms")
        .to_owned()
}
