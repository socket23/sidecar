//! We can create a new session over here and its composed of exchanges
//! The exchanges can be made by the human or the agent

use crate::{
    agentic::symbol::events::message_event::SymbolEventMessageProperties, repo::types::RepoRef,
    user_context::types::UserContext,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExchangeReply {
    Chat,
    Plan,
    AnchoredEdit,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExchangeType {
    HumanChat(ExchangeTypeHuman),
    AgentChat(String),
    AnchoredEdit(String),
    Plan(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeTypeHuman {
    query: String,
    user_context: UserContext,
    project_labels: Vec<String>,
    repo_ref: RepoRef,
}

impl ExchangeTypeHuman {
    pub fn new(
        query: String,
        user_context: UserContext,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
    ) -> Self {
        Self {
            query,
            user_context,
            project_labels,
            repo_ref,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Exchange {
    exchange_id: String,
    exchange_type: ExchangeType,
}

impl Exchange {
    pub fn human_chat(
        exchange_id: String,
        query: String,
        user_context: UserContext,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
    ) -> Self {
        Self {
            exchange_id,
            exchange_type: ExchangeType::HumanChat(ExchangeTypeHuman::new(
                query,
                user_context,
                project_labels,
                repo_ref,
            )),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    id: String,
    exchanges: Vec<Exchange>,
}

impl Session {
    pub fn new(id: String) -> Self {
        Self {
            id,
            exchanges: vec![],
        }
    }

    pub async fn human_message(
        &mut self,
        exchange_id: String,
        human_message: String,
        user_context: UserContext,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
    ) {
        self.exchanges.push(Exchange::human_chat(
            exchange_id,
            human_message,
            user_context,
            project_labels,
            repo_ref,
        ));
    }

    /// This reacts to the last message and generates the reply for the user to handle
    pub async fn reply_to_last_message(
        &mut self,
        exchange_reply: ExchangeReply,
        _message_properties: SymbolEventMessageProperties,
    ) {
        match exchange_reply {
            ExchangeReply::Chat => {}
            ExchangeReply::Plan => {}
            ExchangeReply::AnchoredEdit => {}
        }
    }

    pub async fn generate_plan_from_last_message(&mut self) {}
}
