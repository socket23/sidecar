//! We can create a new session over here and its composed of exchanges
//! The exchanges can be made by the human or the agent

use std::sync::Arc;

use crate::{
    agentic::{
        symbol::{
            errors::SymbolError,
            events::{human::HumanAgenticRequest, message_event::SymbolEventMessageProperties},
            scratch_pad::ScratchPadAgent,
            tool_box::ToolBox,
            ui_event::UIEventWithID,
        },
        tool::{input::ToolInput, r#type::Tool},
    },
    chunking::text_document::Range,
    repo::types::RepoRef,
    user_context::types::UserContext,
};

use super::chat::{SessionChatClientRequest, SessionChatMessage};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AideAgentMode {
    Edit = 1,
    Plan = 2,
    Chat = 3,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AideEditMode {
    Anchored = 1,
    Agentic = 2,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExchangeType {
    HumanChat(ExchangeTypeHuman),
    AgentChat(String),
    // what do we store over here for the anchored edit, it can't just be the
    // user query? we probably have to store the snippet we were trying to edit
    // as well
    Edit(ExchangeTypeEdit),
    Plan(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeEditInformationAgentic {
    query: String,
    codebase_search: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeEditInformationAnchored {
    query: String,
    fs_file_path: String,
    range: Range,
    selection_context: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExchangeEditInformation {
    Agentic(ExchangeEditInformationAgentic),
    Anchored(ExchangeEditInformationAnchored),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeTypeEdit {
    information: ExchangeEditInformation,
    user_context: UserContext,
    exchange_type: AideEditMode,
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
    fn human_chat(
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

    fn agentic_edit(
        exchange_id: String,
        query: String,
        codebase_search: bool,
        user_context: UserContext,
    ) -> Self {
        Self {
            exchange_id,
            exchange_type: ExchangeType::Edit(ExchangeTypeEdit {
                information: ExchangeEditInformation::Agentic(ExchangeEditInformationAgentic {
                    query,
                    codebase_search,
                }),
                user_context,
                exchange_type: AideEditMode::Agentic,
            }),
        }
    }

    fn anchored_edit(
        exchange_id: String,
        query: String,
        user_context: UserContext,
        range: Range,
        fs_file_path: String,
        selection_context: String,
    ) -> Self {
        Self {
            exchange_id,
            exchange_type: ExchangeType::Edit(ExchangeTypeEdit {
                information: ExchangeEditInformation::Anchored(ExchangeEditInformationAnchored {
                    query,
                    fs_file_path,
                    range,
                    selection_context,
                }),
                user_context,
                exchange_type: AideEditMode::Anchored,
            }),
        }
    }

    fn agent_reply(exchange_id: String, message: String) -> Self {
        Self {
            exchange_id,
            exchange_type: ExchangeType::AgentChat(message),
        }
    }

    /// Convert the exchange to a session chat message so we can send it over
    /// for inference
    ///
    /// We can have consecutive human messages now on every API so this is no
    /// longer a big worry
    async fn to_conversation_message(&self) -> SessionChatMessage {
        match &self.exchange_type {
            ExchangeType::HumanChat(ref chat_message) => {
                // TODO(skcd): Figure out caching etc later on
                let user_context = chat_message
                    .user_context
                    .clone()
                    .to_xml(Default::default())
                    .await
                    .unwrap_or_default();
                let prompt = chat_message.query.to_owned();
                SessionChatMessage::user(format!(
                    r#"<attached_context>
{user_context}
</attached_context>
{prompt}"#
                ))
            }
            ExchangeType::AgentChat(ref chat_message) => {
                SessionChatMessage::assistant(chat_message.to_string())
            }
            ExchangeType::Plan(_plan) => {
                todo!("plan branch not impmlemented yet")
            }
            ExchangeType::Edit(_anchored_edit) => {
                todo!("anchored_edit branch not implemented yet")
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    session_id: String,
    project_labels: Vec<String>,
    repo_ref: RepoRef,
    exchanges: Vec<Exchange>,
    storage_path: String,
}

impl Session {
    pub fn new(
        session_id: String,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
        storage_path: String,
    ) -> Self {
        Self {
            session_id,
            project_labels,
            repo_ref,
            exchanges: vec![],
            storage_path,
        }
    }

    pub fn storage_path(&self) -> &str {
        &self.storage_path
    }

    pub fn exchanges(&self) -> usize {
        self.exchanges.len()
    }

    pub fn agentic_edit(
        mut self,
        exchange_id: String,
        query: String,
        user_context: UserContext,
        codebase_search: bool,
    ) -> Session {
        let exchange = Exchange::agentic_edit(exchange_id, query, codebase_search, user_context);
        self.exchanges.push(exchange);
        self
    }

    pub fn anchored_edit(
        mut self,
        exchange_id: String,
        query: String,
        user_context: UserContext,
        range: Range,
        fs_file_path: String,
        file_content_in_selection: String,
    ) -> Session {
        let exchange = Exchange::anchored_edit(
            exchange_id,
            query,
            user_context,
            range,
            fs_file_path,
            file_content_in_selection,
        );
        self.exchanges.push(exchange);
        self
    }

    pub fn human_message(
        mut self,
        exchange_id: String,
        human_message: String,
        user_context: UserContext,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
    ) -> Session {
        let exchange = Exchange::human_chat(
            exchange_id,
            human_message,
            user_context,
            project_labels,
            repo_ref,
        );
        self.exchanges.push(exchange);
        self
    }

    fn last_exchange(&self) -> Option<&Exchange> {
        self.exchanges.last()
    }

    /// This reacts to the last message and generates the reply for the user to handle
    ///
    /// we should have a way to sync this up with a queue based system so we react to events
    /// one after another
    pub async fn reply_to_last_exchange(
        self,
        exchange_reply: AideAgentMode,
        tool_box: Arc<ToolBox>,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Self, SymbolError> {
        let last_exchange = self.last_exchange();
        if last_exchange.is_none() {
            return Ok(self);
        }
        match exchange_reply {
            AideAgentMode::Chat => self.chat_reply(tool_box, message_properties).await,
            AideAgentMode::Plan => {
                todo!("plan branch")
            }
            AideAgentMode::Edit => {
                todo!("edit branch not supported")
            }
        }
    }

    /// This sends back a reply to the user message, using the context we have from before
    async fn chat_reply(
        self,
        tool_box: Arc<ToolBox>,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Self, SymbolError> {
        println!("session::chat_reply");
        // over here we want to convert all the previous exchanges to a context prompt
        // and then generate the appropriate things required
        let last_exchange = self.last_exchange();
        if last_exchange.is_none() {
            return Ok(self);
        }
        let last_exchange = last_exchange.expect("is_none to hold").clone();
        let session_id = self.session_id.to_owned();
        // - we have to grab a new exchange id over here before we start sending the reply over
        let response_exchange_id = tool_box
            .create_new_exchange(session_id, message_properties.clone())
            .await?;

        // Now that we have a new response exchange id we want to start streaming the reply back
        // to the user
        let last_exchange_type = last_exchange.exchange_type;
        match last_exchange_type {
            ExchangeType::HumanChat(_) => {
                self.human_chat_message_reply(response_exchange_id, tool_box, message_properties)
                    .await
            }
            ExchangeType::AgentChat(_agent_message) => {
                todo!("figure out what to do over here")
            }
            ExchangeType::Edit(_edit) => {
                todo!("figure out what to do over here")
            }
            ExchangeType::Plan(_plan) => {
                todo!("figure out what to do over here")
            }
        }
    }

    /// Create the stream which will reply to the user
    async fn human_chat_message_reply(
        mut self,
        exchange_id: String,
        tool_box: Arc<ToolBox>,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Session, SymbolError> {
        println!("session::human_chat_message_reply");
        // take everything until the exchange id of the message we are supposed to
        // reply to
        let mut converted_messages = vec![];
        for previous_message in self.exchanges.iter() {
            converted_messages.push(previous_message.to_conversation_message().await);
        }

        let tool_input = SessionChatClientRequest::new(
            tool_box
                .recently_edited_files(Default::default(), message_properties.clone())
                .await?,
            UserContext::new(vec![], vec![], None, vec![]),
            converted_messages,
            self.repo_ref.clone(),
            self.project_labels.to_vec(),
            self.session_id.to_owned(),
            exchange_id.to_owned(),
            message_properties.ui_sender(),
        );
        let chat_output = tool_box
            .tools()
            .invoke(ToolInput::ContextDrivenChatReply(tool_input))
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .context_drive_chat_reply()
            .ok_or(SymbolError::WrongToolOutput)?
            .reply();
        self.exchanges
            .push(Exchange::agent_reply(exchange_id.to_owned(), chat_output));
        let ui_sender = message_properties.ui_sender();
        // finsihed the exchange here since we have replied already
        let _ = ui_sender.send(UIEventWithID::finished_exchange(
            self.session_id.to_owned(),
            exchange_id,
        ));
        Ok(self)
    }

    /// we are going to perform the agentic editing
    pub async fn perform_agentic_editing(
        mut self,
        scratch_pad_agent: ScratchPadAgent,
        root_directory: String,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Self, SymbolError> {
        let last_exchange = self.last_exchange();
        if let Some(Exchange {
            exchange_id: _,
            exchange_type:
                ExchangeType::Edit(ExchangeTypeEdit {
                    information:
                        ExchangeEditInformation::Agentic(ExchangeEditInformationAgentic {
                            query,
                            codebase_search,
                        }),
                    user_context,
                    ..
                }),
        }) = last_exchange
        {
            let edits_performed = scratch_pad_agent
                .human_message_agentic(
                    HumanAgenticRequest::new(
                        query.to_owned(),
                        root_directory,
                        *codebase_search,
                        user_context.clone(),
                        false,
                    ),
                    message_properties.clone(),
                )
                .await;
            let message = match edits_performed {
                Ok(_) => {
                    // add a message to the same exchange that we are done
                    "Finished editing".to_owned()
                }
                Err(_) => "Failed to edit".to_owned(),
            };

            // Send a reply on the exchange
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::chat_event(
                    message_properties.root_request_id().to_owned(),
                    message_properties.request_id_str().to_owned(),
                    message.to_owned(),
                    Some(message.to_owned()),
                ));

            // Add to the exchange
            self.exchanges.push(Exchange::agent_reply(
                message_properties.request_id_str().to_owned(),
                message.to_owned(),
            ));

            // now close the exchange
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::finished_exchange(
                    self.session_id.to_owned(),
                    message_properties.request_id_str().to_owned(),
                ));
        }
        Ok(self)
    }

    /// We perform the anchored edit over here
    pub async fn perform_anchored_edit(
        mut self,
        scratch_pad_agent: ScratchPadAgent,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Self, SymbolError> {
        let last_exchange = self.last_exchange();
        if let Some(Exchange {
            exchange_id: _,
            exchange_type:
                ExchangeType::Edit(ExchangeTypeEdit {
                    information:
                        ExchangeEditInformation::Anchored(ExchangeEditInformationAnchored {
                            query,
                            fs_file_path,
                            range,
                            selection_context: _,
                        }),
                    user_context,
                    ..
                }),
        }) = last_exchange
        {
            let edits_performed = scratch_pad_agent
                .anchor_editing_on_range(
                    range.clone(),
                    fs_file_path.to_owned(),
                    query.to_owned(),
                    user_context
                        .clone()
                        .to_xml(Default::default())
                        .await
                        .map_err(|e| SymbolError::UserContextError(e))?,
                    message_properties.clone(),
                )
                .await;
            let message = match edits_performed {
                Ok(_) => {
                    // add a message to the same exchange that we are done
                    "Finished editing".to_owned()
                }
                Err(_) => "Failed to edit".to_owned(),
            };

            // Send a reply on the exchange
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::chat_event(
                    message_properties.root_request_id().to_owned(),
                    message_properties.request_id_str().to_owned(),
                    message.to_owned(),
                    Some(message.to_owned()),
                ));

            // Add to the exchange
            self.exchanges.push(Exchange::agent_reply(
                message_properties.request_id_str().to_owned(),
                message.to_owned(),
            ));

            // now close the exchange
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::finished_exchange(
                    self.session_id.to_owned(),
                    message_properties.request_id_str().to_owned(),
                ));
        }
        Ok(self)
    }
}
