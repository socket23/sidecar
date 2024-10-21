//! We can create a new session over here and its composed of exchanges
//! The exchanges can be made by the human or the agent

use std::sync::Arc;

use futures::StreamExt;

use crate::{
    agentic::{
        symbol::{
            errors::SymbolError,
            events::{
                edit::SymbolToEdit,
                human::HumanAgenticRequest,
                message_event::{SymbolEventMessage, SymbolEventMessageProperties},
            },
            identifier::SymbolIdentifier,
            manager::SymbolManager,
            scratch_pad::ScratchPadAgent,
            tool_box::ToolBox,
            tool_properties::ToolProperties,
            types::SymbolEventRequest,
            ui_event::UIEventWithID,
        },
        tool::{
            input::ToolInput,
            plan::{generator::StepSenderEvent, service::PlanService},
            r#type::Tool,
        },
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

/// The exchange can be in one of the states
///
/// Its either that the edits made were accepted or rejected
/// it could also have been cancelled by the user
/// Default when its created is in running state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExchangeState {
    Accepted,
    Rejected,
    Cancelled,
    Running,
    UserMessage,
}

impl Default for ExchangeState {
    fn default() -> Self {
        ExchangeState::Running
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExchangeType {
    HumanChat(ExchangeTypeHuman),
    AgentChat(String),
    // what do we store over here for the anchored edit, it can't just be the
    // user query? we probably have to store the snippet we were trying to edit
    // as well
    Edit(ExchangeTypeEdit),
    Plan(ExchangeTypePlan),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeTypePlan {
    query: String,
    user_context: UserContext,
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
    #[serde(default)]
    exchange_state: ExchangeState,
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
            exchange_state: ExchangeState::UserMessage,
        }
    }

    fn plan_request(exchange_id: String, query: String, user_context: UserContext) -> Self {
        Self {
            exchange_id,
            exchange_type: ExchangeType::Plan(ExchangeTypePlan {
                query,
                user_context,
            }),
            exchange_state: ExchangeState::UserMessage,
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
            exchange_state: ExchangeState::UserMessage,
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
            exchange_state: ExchangeState::UserMessage,
        }
    }

    fn agent_reply(exchange_id: String, message: String) -> Self {
        Self {
            exchange_id,
            exchange_type: ExchangeType::AgentChat(message),
            exchange_state: ExchangeState::Running,
        }
    }

    fn set_completion_status(mut self, accetped: bool) -> Self {
        if accetped {
            self.exchange_state = ExchangeState::Accepted;
        } else {
            self.exchange_state = ExchangeState::Rejected;
        }
        self
    }

    /// If the exchange has been left open and has not finished yet
    fn is_open(&self) -> bool {
        matches!(self.exchange_state, ExchangeState::Running)
            && matches!(self.exchange_type, ExchangeType::AgentChat(_))
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

    pub fn plan(
        mut self,
        exchange_id: String,
        query: String,
        user_context: UserContext,
    ) -> Session {
        let exchange = Exchange::plan_request(exchange_id, query, user_context);
        self.exchanges.push(exchange);
        self
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

    pub async fn react_to_feedback(
        mut self,
        exchange_id: &str,
        accepted: bool,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Self, SymbolError> {
        // Here first we make sure that an exchange of the form exists
        // if it does we mark that exchange as closed and also update its state
        self.exchanges = self
            .exchanges
            .into_iter()
            .map(|exchange| {
                if &exchange.exchange_id == exchange_id {
                    // we have an exchange over here matching our id so update its state
                    // to what it is
                    exchange.set_completion_status(accepted)
                } else {
                    exchange
                }
            })
            .collect();

        // give feedback to the editor that our state has chagned
        if accepted {
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::edits_accepted(
                    self.session_id.to_owned(),
                    exchange_id.to_owned(),
                ));
        } else {
            let _ =
                message_properties
                    .ui_sender()
                    .send(UIEventWithID::edits_cancelled_in_exchange(
                        self.session_id.to_owned(),
                        exchange_id.to_owned(),
                    ));
        }

        // now close the exchange
        let _ = message_properties
            .ui_sender()
            .send(UIEventWithID::finished_exchange(
                self.session_id.to_owned(),
                message_properties.request_id_str().to_owned(),
            ));
        Ok(self)
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

        // Now that we have a new response exchange id we want to start streaming the reply back
        // to the user
        let last_exchange_type = last_exchange.exchange_type;
        match last_exchange_type {
            ExchangeType::HumanChat(_) => {
                self.human_chat_message_reply(tool_box, message_properties)
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

        let exchange_id = message_properties.request_id_str().to_owned();

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
            message_properties.cancellation_token(),
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

    /// We want to make sure that any open exchanges are accepted as we make
    /// progress towards are current exchange
    pub fn accept_open_exchanges_if_any(
        mut self,
        message_properties: SymbolEventMessageProperties,
    ) -> Self {
        let exchanges_to_close = self
            .exchanges
            .iter()
            .filter_map(|exchange| {
                if exchange.is_open() {
                    Some(exchange.exchange_id.to_owned())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        exchanges_to_close.into_iter().for_each(|exchange_id| {
            // mark the exchange as accepted
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::edits_accepted(
                    self.session_id.to_owned(),
                    exchange_id.to_owned(),
                ));
            // mark the exchange as closed
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::finished_exchange(
                    self.session_id.to_owned(),
                    exchange_id,
                ));
        });

        // now update all our exchanges to accepted
        self.exchanges = self
            .exchanges
            .into_iter()
            .map(|exchange| {
                if exchange.is_open() {
                    exchange.set_completion_status(true)
                } else {
                    exchange
                }
            })
            .collect();

        self
    }

    /// We have to map the plan revert exchange-id over here to be similar to
    /// the previous plan exchange-id, doing this will allow us to make sure
    /// that we are able to keep track of the edits properly
    pub async fn perform_plan_revert(
        mut self,
        plan_service: PlanService,
        previous_plan_exchange_id: String,
        step_index: usize,
        tool_box: Arc<ToolBox>,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Self, SymbolError> {
        let original_plan = plan_service
            .load_plan_from_id(
                &plan_service.generate_unique_plan_id(&self.session_id, &previous_plan_exchange_id),
            )
            .await
            .map_err(|e| SymbolError::IOError(e))?;

        let exchange_id = message_properties.request_id_str().to_owned();

        if step_index == 0 {
            let ui_sender = message_properties.ui_sender();
            // revert the changes back by talking to the editor
            let _ = tool_box
                .undo_changes_made_during_session(
                    self.session_id.to_owned(),
                    previous_plan_exchange_id.to_owned(),
                    Some(step_index),
                    message_properties.clone(),
                )
                .await;

            let reply =
                "I have reverted the full plan, let me know how I can be of help?".to_owned();
            let _ = ui_sender.send(UIEventWithID::chat_event(
                self.session_id.to_owned(),
                exchange_id.to_owned(),
                reply.to_owned(),
                Some(reply.to_owned()),
            ));

            // update our exchanges and add what we did
            self.exchanges.push(Exchange::agent_reply(
                message_properties.request_id_str().to_owned(),
                reply,
            ));

            // now close the exchange
            let _ = ui_sender.send(UIEventWithID::finished_exchange(
                self.session_id.to_owned(),
                exchange_id,
            ));
        } else {
            let updated_plan = original_plan.drop_plan_steps(step_index);
            let ui_sender = message_properties.ui_sender();
            // send all the updated plan steps to the exchange
            updated_plan
                .steps()
                .into_iter()
                .enumerate()
                .filter(|(idx, _)| *idx < step_index)
                .for_each(|(idx, plan_step)| {
                    let _ = ui_sender.send(UIEventWithID::plan_complete_added(
                        self.session_id.to_owned(),
                        previous_plan_exchange_id.to_owned(),
                        idx,
                        plan_step.files_to_edit().to_vec(),
                        plan_step.title().to_owned(),
                        plan_step.description().to_owned(),
                    ));
                });

            // revert the changes back by talking to the editor
            let _ = tool_box
                .undo_changes_made_during_session(
                    self.session_id.to_owned(),
                    previous_plan_exchange_id.to_owned(),
                    Some(step_index),
                    message_properties.clone(),
                )
                .await;

            // now send a message to the exchange telling that we have reverted
            // the changes
            let _ = ui_sender.send(UIEventWithID::chat_event(
                self.session_id.to_owned(),
                exchange_id.to_owned(),
                "I have reverted the changes made by the plan".to_owned(),
                Some("I have reverted the changes made by the plan".to_owned()),
            ));

            // update our exchanges and add what we did
            self.exchanges.push(Exchange::agent_reply(
                message_properties.request_id_str().to_owned(),
                "I have reverted the changes made by the plan".to_owned(),
            ));

            // now close the exchange
            let _ = ui_sender.send(UIEventWithID::finished_exchange(
                self.session_id.to_owned(),
                exchange_id,
            ));
        }

        Ok(self)
    }

    /// going to work on plan generation
    pub async fn perform_plan_generation(
        mut self,
        plan_service: PlanService,
        plan_id: String,
        plan_storage_path: String,
        symbol_manager: Arc<SymbolManager>,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<Self, SymbolError> {
        let last_exchange = self.last_exchange().cloned();
        if let Some(Exchange {
            exchange_id: _,
            exchange_type:
                ExchangeType::Plan(ExchangeTypePlan {
                    query,
                    user_context,
                }),
            exchange_state: _,
        }) = last_exchange
        {
            // since we are going to be streaming the steps over here as we also
            // have to very quickly start editing the files as the steps are coming
            // in, in a very optimistic way
            let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

            let mut stream_receiver =
                tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);

            // now we want to poll from the receiver over here and start reacting to
            // the events
            let cloned_message_properties = message_properties.clone();
            let cloned_plan_service = plan_service.clone();
            let plan = tokio::spawn(async move {
                cloned_plan_service
                    .create_plan(
                        plan_id,
                        query.to_owned(),
                        user_context.clone(),
                        false, // deep reasoning toggle, set to false right now by default
                        plan_storage_path,
                        Some(sender),
                        cloned_message_properties.clone(),
                    )
                    .await
            });

            let mut steps_up_until_now = 0;
            while let Some(step_message) = stream_receiver.next().await {
                match step_message {
                    StepSenderEvent::NewStep(step) => {
                        println!("session::perform_plan_generation::new_step_found");
                        let instruction = step.description();
                        let file_to_edit = step.file_to_edit();
                        if file_to_edit.is_none() {
                            continue;
                        }
                        let file_to_edit = file_to_edit.expect("is_none to hold");
                        let hub_sender = symbol_manager.hub_sender();
                        let (edit_done_sender, edit_done_receiver) =
                            tokio::sync::oneshot::channel();
                        let _ = hub_sender.send(SymbolEventMessage::new(
                            SymbolEventRequest::simple_edit_request(
                                SymbolIdentifier::with_file_path(&file_to_edit, &file_to_edit),
                                SymbolToEdit::new(
                                    file_to_edit.to_owned(),
                                    Range::default(),
                                    file_to_edit.to_owned(),
                                    vec![instruction.to_owned()],
                                    false,
                                    false,
                                    true,
                                    instruction.to_owned(),
                                    None,
                                    false,
                                    None,
                                    true,
                                    None,
                                    vec![],
                                    Some(steps_up_until_now.to_string()),
                                ),
                                ToolProperties::new(),
                            ),
                            message_properties.request_id().clone(),
                            message_properties.ui_sender().clone(),
                            edit_done_sender,
                            message_properties.cancellation_token(),
                            message_properties.editor_url(),
                        ));

                        // increment our count for the step over here
                        steps_up_until_now = steps_up_until_now + 1;

                        // wait for the edits to finish over here
                        let _ = edit_done_receiver.await;
                    }
                    StepSenderEvent::Done => {
                        break;
                    }
                }
            }
            // close the receiver stream since we are no longer interested in any
            // of the events after getting a done event
            stream_receiver.close();
            // we will start polling from the receiver soon
            println!("session::perform_plan_generation::finished_plan_generation");
            // we have to also start working on top of the plan after this
            let message = match plan.await {
                Ok(Ok(plan)) => {
                    let _ = plan_service.mark_plan_completed(plan).await;
                    "Generated plan".to_owned()
                }
                _ => "Failed to generate plan".to_owned(),
            };

            // send a reply on the exchange
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
            println!("session::finished_plan_generation");
        }
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
            exchange_state: _,
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
            println!("session::perform_agentic_editing::finsihed_editing");
            let message = match edits_performed {
                Ok(_) => {
                    // add a message to the same exchange that we are done
                    "Finished editing".to_owned()
                }
                Err(_) => "Failed to edit".to_owned(),
            };

            // Send a reply on the exchange
            // let _ = message_properties
            //     .ui_sender()
            //     .send(UIEventWithID::chat_event(
            //         message_properties.root_request_id().to_owned(),
            //         message_properties.request_id_str().to_owned(),
            //         message.to_owned(),
            //         Some(message.to_owned()),
            //     ));

            // Add to the exchange
            self.exchanges.push(Exchange::agent_reply(
                message_properties.request_id_str().to_owned(),
                message.to_owned(),
            ));

            // Also tell the exchange that we are in review mode now
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::edits_in_review(
                    message_properties.root_request_id().to_owned(),
                    message_properties.request_id_str().to_owned(),
                ));
            println!("session::finished_agentic_editing_exchange");
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
            exchange_state: _,
        }) = last_exchange
        {
            // send a message that we are starting with the edits over here
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::edits_started_in_exchnage(
                    message_properties.root_request_id().to_owned(),
                    message_properties.request_id_str().to_owned(),
                ));
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

            // Also tell the exchange that we are in review mode now
            let _ = message_properties
                .ui_sender()
                .send(UIEventWithID::edits_in_review(
                    message_properties.root_request_id().to_owned(),
                    message_properties.request_id_str().to_owned(),
                ));

            // Add to the exchange
            self.exchanges.push(Exchange::agent_reply(
                message_properties.request_id_str().to_owned(),
                message.to_owned(),
            ));
        }
        Ok(self)
    }
}
