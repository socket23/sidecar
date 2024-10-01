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

pub async fn append_to_plan(
    plan_id: uuid::Uuid,
    plan_storage_path: String,
    plan_service: PlanService,
    query: String,
    user_context: UserContext,
    message_properties: SymbolEventMessageProperties,
    agent_sender: UnboundedSender<anyhow::Result<ConversationMessage>>,
) {
    let plan = plan_service.load_plan(&plan_storage_path).await;
    if let Err(_) = plan {
        let final_answer = "failed to load plan from storage".to_owned();
        let _ = agent_sender.send(Ok(ConversationMessage::answer_update(
            plan_id,
            AgentAnswerStreamEvent::LLMAnswer(LLMClientCompletionResponse::new(
                final_answer.to_owned(),
                Some(final_answer.to_owned()),
                "Custom".to_owned(),
            )),
        )));
        return;
    }
    let plan = plan.expect("plan to be present");
    if let Ok(plan) = plan_service
        .append_step(plan, query, user_context, message_properties)
        .await
    {
        let _ = plan_service.save_plan(&plan, &plan_storage_path).await;
    } else {
        // errored to update the plan
    }
}

/// Executes the plan until a checkpoint
pub async fn execute_plan_until(
    // the checkpoint until which we want to execute the plan
    execute_until: usize,
    plan_id: uuid::Uuid,
    plan_storage_path: String,
    plan_service: PlanService,
    message_properties: SymbolEventMessageProperties,
    agent_sender: UnboundedSender<anyhow::Result<ConversationMessage>>,
) {
    // loads the plan from a storage location
    let plan = plan_service.load_plan(&plan_storage_path).await;
    if let Err(_) = plan {
        let final_answer = "failed to load plan from stroage".to_owned();
        let _ = agent_sender.send(Ok(ConversationMessage::answer_update(
            plan_id,
            AgentAnswerStreamEvent::LLMAnswer(LLMClientCompletionResponse::new(
                final_answer.to_owned(),
                Some(final_answer.to_owned()),
                "Custom".to_owned(),
            )),
        )));
        return;
    }
    let mut plan = plan.expect("plan to be present");
    for (idx, plan_step) in plan
        .steps()
        .to_vec()
        .iter()
        .enumerate()
        .filter_map(|(idx, step)| {
            if idx <= execute_until {
                Some((idx, step))
            } else {
                None
            }
        })
    {
        if plan.checkpoint().is_some() && idx <= plan.checkpoint().unwrap_or_default() {
            let executing_step = format!(
                "Already executed step:{}, checkpoint is at: {}",
                idx,
                plan.checkpoint().unwrap_or_default()
            );
            let _ = agent_sender.send(Ok(ConversationMessage::answer_update(
                plan_id,
                AgentAnswerStreamEvent::LLMAnswer(LLMClientCompletionResponse::new(
                    executing_step.to_owned(),
                    Some(executing_step.to_owned()),
                    "Custom".to_owned(),
                )),
            )));
            continue;
        }
        // starting executing each step over here
        let checkpoint = plan.checkpoint().unwrap_or_default();
        let context = plan_service.prepare_context(plan.steps(), checkpoint).await;
        let execution_result = plan_service
            .execute_step(plan_step, context, message_properties.clone())
            .await;
        if let Err(_) = execution_result {
            let _ = agent_sender.send(Ok(ConversationMessage::answer_update(
                plan_id,
                AgentAnswerStreamEvent::LLMAnswer(LLMClientCompletionResponse::new(
                    format!("Errored out while executing step: {}", idx).to_owned(),
                    Some(format!("Errored out while executing step: {}", idx).to_owned()),
                    "Custom".to_owned(),
                )),
            )));
            return;
        }
        let _ = agent_sender.send(Ok(ConversationMessage::answer_update(
            plan_id,
            AgentAnswerStreamEvent::LLMAnswer(LLMClientCompletionResponse::new(
                format!("Finished executing until: {}", idx).to_owned(),
                Some(format!("Finished executing until: {}", idx).to_owned()),
                "Custom".to_owned(),
            )),
        )));
        let _ = plan.increment_checkpoint();
        // save the updated checkpoint in the storage layer
        let _ = plan_service.save_plan(&plan, &plan_storage_path).await;
    }
}

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

pub async fn handle_execute_plan_until(
    execute_until: usize,
    plan_id: uuid::Uuid,
    plan_storage_path: String,
    editor_url: String,
    plan_service: PlanService,
) -> Result<
    Sse<std::pin::Pin<Box<dyn tokio_stream::Stream<Item = anyhow::Result<sse::Event>> + Send>>>,
> {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let (ui_sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let plan_id_str = plan_id.to_string();
    let message_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new(plan_id_str.to_owned(), plan_id_str.to_owned()),
        ui_sender,
        editor_url,
        cancellation_token,
    );

    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let _ = tokio::spawn(async move {
        execute_plan_until(
            execute_until,
            plan_id,
            plan_storage_path,
            plan_service,
            message_properties,
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
