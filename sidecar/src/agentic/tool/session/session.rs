//! We can create a new session over here and its composed of exchanges
//! The exchanges can be made by the human or the agent

use std::sync::Arc;

use crate::{
    agentic::symbol::{events::message_event::SymbolEventMessageProperties, tool_box::ToolBox},
    repo::types::RepoRef,
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

    fn last_exchange(&self) -> Option<&Exchange> {
        self.exchanges.last()
    }

    /// This reacts to the last message and generates the reply for the user to handle
    ///
    /// we should have a way to sync this up with a queue based system so we react to events
    /// one after another
    pub async fn reply_to_last_exchange(
        mut self,
        exchange_reply: ExchangeReply,
        tool_box: Arc<ToolBox>,
        message_properties: SymbolEventMessageProperties,
    ) {
        let last_exchange = self.last_exchange();
        if last_exchange.is_none() {
            return;
        }
        match exchange_reply {
            ExchangeReply::Chat => self.chat_reply(tool_box, message_properties).await,
            ExchangeReply::Plan => {}
            ExchangeReply::AnchoredEdit => {}
        }
    }

    /// This sends back a reply to the user message, using the context we have from before
    async fn chat_reply(
        &mut self,
        tool_box: Arc<ToolBox>,
        message_properties: SymbolEventMessageProperties,
    ) {
        // over here we want to convert all the previous exchanges to a context prompt
        // and then generate the appropriate things required
        // - we have to grab a new exchange id over here before we start sending the reply over
    }
}
