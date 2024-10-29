//! Creates the service which handles saving the session and extending it

use std::{collections::HashMap, sync::Arc};

use tokio::{io::AsyncWriteExt, sync::Mutex};
use tokio_util::sync::CancellationToken;

use crate::{
    agentic::{
        symbol::{
            errors::SymbolError, events::message_event::SymbolEventMessageProperties,
            manager::SymbolManager, scratch_pad::ScratchPadAgent, tool_box::ToolBox,
        },
        tool::plan::service::PlanService,
    },
    chunking::text_document::Range,
    repo::types::RepoRef,
    user_context::types::UserContext,
};

use super::session::{AideAgentMode, Session};

/// The session service which takes care of creating the session and manages the storage
pub struct SessionService {
    tool_box: Arc<ToolBox>,
    symbol_manager: Arc<SymbolManager>,
    running_exchanges: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl SessionService {
    pub fn new(tool_box: Arc<ToolBox>, symbol_manager: Arc<SymbolManager>) -> Self {
        Self {
            tool_box,
            symbol_manager,
            running_exchanges: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn track_exchange(
        &self,
        session_id: &str,
        exchange_id: &str,
        cancellation_token: CancellationToken,
    ) {
        let hash_id = format!("{}-{}", session_id, exchange_id);
        let mut running_exchanges = self.running_exchanges.lock().await;
        running_exchanges.insert(hash_id, cancellation_token);
    }

    pub async fn get_cancellation_token(
        &self,
        session_id: &str,
        exchange_id: &str,
    ) -> Option<CancellationToken> {
        let hash_id = format!("{}-{}", session_id, exchange_id);
        let running_exchanges = self.running_exchanges.lock().await;
        running_exchanges
            .get(&hash_id)
            .map(|cancellation_token| cancellation_token.clone())
    }

    fn create_new_session(
        &self,
        session_id: String,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
        storage_path: String,
        global_user_context: UserContext,
    ) -> Session {
        Session::new(
            session_id,
            project_labels,
            repo_ref,
            storage_path,
            global_user_context,
        )
    }

    pub async fn human_message(
        &self,
        session_id: String,
        storage_path: String,
        exchange_id: String,
        human_message: String,
        user_context: UserContext,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
        agent_mode: AideAgentMode,
        mut message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        println!("session_service::human_message::start");
        let mut session = if let Ok(session) = self.load_from_storage(storage_path.to_owned()).await
        {
            println!(
                "session_service::load_from_storage_ok::session_id({})",
                &session_id
            );
            session
        } else {
            self.create_new_session(
                session_id.to_owned(),
                project_labels.to_vec(),
                repo_ref.clone(),
                storage_path,
                user_context.clone(),
            )
        };

        println!("session_service::session_created");

        // add human message
        session = session.human_message(
            exchange_id,
            human_message,
            user_context,
            project_labels,
            repo_ref,
        );

        let plan_exchange_id = self
            .tool_box
            .create_new_exchange(session_id.to_owned(), message_properties.clone())
            .await?;

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        self.track_exchange(&session_id, &plan_exchange_id, cancellation_token.clone())
            .await;
        message_properties = message_properties
            .set_request_id(plan_exchange_id)
            .set_cancellation_token(cancellation_token);

        // now react to the last message
        session = session
            .reply_to_last_exchange(agent_mode, self.tool_box.clone(), message_properties)
            .await?;

        // save the session to the disk
        self.save_to_storage(&session).await?;
        Ok(())
    }

    /// Generates the plan over here and upon invocation we take care of executing
    /// the steps
    pub async fn plan_generation(
        &self,
        session_id: String,
        storage_path: String,
        plan_storage_path: String,
        plan_id: String,
        plan_service: PlanService,
        exchange_id: String,
        query: String,
        user_context: UserContext,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
        _root_directory: String,
        _codebase_search: bool,
        mut message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        println!("session_service::plan::agentic::start");
        let mut session = if let Ok(session) = self.load_from_storage(storage_path.to_owned()).await
        {
            println!(
                "session_service::load_from_storage_ok::session_id({})",
                &session_id
            );
            session
        } else {
            self.create_new_session(
                session_id.to_owned(),
                project_labels.to_vec(),
                repo_ref.clone(),
                storage_path,
                user_context.clone(),
            )
        };

        // here we first check if we are going to revert and if we are, then we need
        // to go back certain steps
        if let Some((previous_exchange_id, step_index)) = plan_service.should_drop_plan() {
            println!(
                "dropping_plan::exchange_id({})::step_index({:?})",
                &previous_exchange_id, &step_index
            );
            // we first add an exchange that the human has request us to rollback
            // on the plan
            session =
                session.human_message(exchange_id, query, user_context, project_labels, repo_ref);

            let plan_exchange_id = self
                .tool_box
                .create_new_exchange(session_id.to_owned(), message_properties.clone())
                .await?;

            let cancellation_token = tokio_util::sync::CancellationToken::new();
            self.track_exchange(&session_id, &plan_exchange_id, cancellation_token.clone())
                .await;
            message_properties = message_properties
                .set_request_id(plan_exchange_id)
                .set_cancellation_token(cancellation_token);

            session = session
                .perform_plan_revert(
                    plan_service,
                    previous_exchange_id,
                    step_index,
                    self.tool_box.clone(),
                    message_properties,
                )
                .await?;
            // save the session to the disk
            self.save_to_storage(&session).await?;
        } else {
            // add an exchange that we are going to genrate a plan over here
            session = session.plan(exchange_id, query, user_context);
            self.save_to_storage(&session).await?;

            // create a new exchange over here for the plan
            let plan_exchange_id = self
                .tool_box
                .create_new_exchange(session_id.to_owned(), message_properties.clone())
                .await?;

            let cancellation_token = tokio_util::sync::CancellationToken::new();
            self.track_exchange(&session_id, &plan_exchange_id, cancellation_token.clone())
                .await;
            message_properties = message_properties
                .set_request_id(plan_exchange_id)
                .set_cancellation_token(cancellation_token);
            // now we can perform the plan generation over here
            session = session
                .perform_plan_generation(
                    plan_service,
                    plan_id,
                    plan_storage_path,
                    self.tool_box.clone(),
                    self.symbol_manager.clone(),
                    message_properties,
                )
                .await?;
            // save the session to the disk
            self.save_to_storage(&session).await?;
        }

        println!("session_service::plan_generation::stop");
        Ok(())
    }

    pub async fn code_edit_agentic(
        &self,
        session_id: String,
        storage_path: String,
        scratch_pad_agent: ScratchPadAgent,
        exchange_id: String,
        edit_request: String,
        user_context: UserContext,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
        root_directory: String,
        codebase_search: bool,
        mut message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        println!("session_service::code_edit::agentic::start");
        let mut session = if let Ok(session) = self.load_from_storage(storage_path.to_owned()).await
        {
            println!(
                "session_service::load_from_storage_ok::session_id({})",
                &session_id
            );
            session
        } else {
            self.create_new_session(
                session_id.to_owned(),
                project_labels.to_vec(),
                repo_ref.clone(),
                storage_path,
                user_context.clone(),
            )
        };

        // add an exchange that we are going to perform anchored edits
        session = session.agentic_edit(exchange_id, edit_request, user_context, codebase_search);

        session = session.accept_open_exchanges_if_any(message_properties.clone());
        let edit_exchange_id = self
            .tool_box
            .create_new_exchange(session_id.to_owned(), message_properties.clone())
            .await?;

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        self.track_exchange(&session_id, &edit_exchange_id, cancellation_token.clone())
            .await;
        message_properties = message_properties
            .set_request_id(edit_exchange_id)
            .set_cancellation_token(cancellation_token);

        session = session
            .perform_agentic_editing(scratch_pad_agent, root_directory, message_properties)
            .await?;

        // save the session to the disk
        self.save_to_storage(&session).await?;
        println!("session_service::code_edit::agentic::stop");
        Ok(())
    }

    /// We are going to try and do code edit since we are donig anchored edit
    pub async fn code_edit_anchored(
        &self,
        session_id: String,
        storage_path: String,
        scratch_pad_agent: ScratchPadAgent,
        exchange_id: String,
        edit_request: String,
        user_context: UserContext,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
        mut message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        println!("session_service::code_edit::anchored::start");
        let mut session = if let Ok(session) = self.load_from_storage(storage_path.to_owned()).await
        {
            println!(
                "session_service::load_from_storage_ok::session_id({})",
                &session_id
            );
            session
        } else {
            self.create_new_session(
                session_id.to_owned(),
                project_labels.to_vec(),
                repo_ref.clone(),
                storage_path,
                user_context.clone(),
            )
        };

        let selection_variable = user_context.variables.iter().find(|variable| {
            variable.is_selection()
                && !(variable.start_position.line() == 0 && variable.end_position.line() == 0)
        });
        if selection_variable.is_none() {
            return Ok(());
        }
        let selection_variable = selection_variable.expect("is_none to hold above");
        let selection_range = Range::new(
            selection_variable.start_position,
            selection_variable.end_position,
        );
        println!("session_service::selection_range::({:?})", &selection_range);
        let selection_fs_file_path = selection_variable.fs_file_path.to_owned();
        let file_content = self
            .tool_box
            .file_open(
                selection_fs_file_path.to_owned(),
                message_properties.clone(),
            )
            .await?;
        let file_content_in_range = file_content
            .content_in_range(&selection_range)
            .unwrap_or(selection_variable.content.to_owned());

        session = session.accept_open_exchanges_if_any(message_properties.clone());
        let edit_exchange_id = self
            .tool_box
            .create_new_exchange(session_id.to_owned(), message_properties.clone())
            .await?;

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        self.track_exchange(&session_id, &edit_exchange_id, cancellation_token.clone())
            .await;
        message_properties = message_properties
            .set_request_id(edit_exchange_id)
            .set_cancellation_token(cancellation_token);

        // add an exchange that we are going to perform anchored edits
        session = session.anchored_edit(
            exchange_id,
            edit_request,
            user_context,
            selection_range,
            selection_fs_file_path,
            file_content_in_range,
        );

        // Now we can start editing the selection over here
        session = session
            .perform_anchored_edit(scratch_pad_agent, message_properties)
            .await?;

        // save the session to the disk
        self.save_to_storage(&session).await?;
        println!("session_service::code_edit::anchored_edit::finished");
        Ok(())
    }

    pub async fn handle_session_undo(
        &self,
        exchange_id: &str,
        storage_path: String,
    ) -> Result<(), SymbolError> {
        let session_maybe = self.load_from_storage(storage_path.to_owned()).await;
        if session_maybe.is_err() {
            return Ok(());
        }
        let mut session = session_maybe.expect("is_err to hold");
        session = session.undo_including_exchange_id(&exchange_id).await?;
        self.save_to_storage(&session).await?;
        Ok(())
    }

    /// Provied feedback to the exchange
    ///
    /// We can react to this later on and send out either another exchange or something else
    /// but for now we are just reacting to it on our side so we know
    pub async fn feedback_for_exchange(
        &self,
        exchange_id: &str,
        step_index: Option<usize>,
        accepted: bool,
        storage_path: String,
        message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        let session_maybe = self.load_from_storage(storage_path.to_owned()).await;
        if session_maybe.is_err() {
            return Ok(());
        }
        let mut session = session_maybe.expect("is_err to hold above");
        session = session
            .react_to_feedback(exchange_id, step_index, accepted, message_properties)
            .await?;
        self.save_to_storage(&session).await?;
        Ok(())
    }

    /// Returns if the exchange was really cancelled
    pub async fn set_exchange_as_cancelled(
        &self,
        storage_path: String,
        exchange_id: String,
    ) -> Result<bool, SymbolError> {
        let mut session = self.load_from_storage(storage_path).await.map_err(|e| {
            println!(
                "session_service::set_exchange_as_cancelled::exchange_id({})::error({:?})",
                &exchange_id, e
            );
            e
        })?;

        let send_cancellation_signal = session.has_running_code_edits(&exchange_id);
        println!(
            "session_service::exchange_id({})::should_cancel::({})",
            &exchange_id, send_cancellation_signal
        );

        let session = if send_cancellation_signal {
            session = session.set_exchange_as_cancelled(&exchange_id);
            session
        } else {
            session
        };

        self.save_to_storage(&session).await?;
        Ok(send_cancellation_signal)
    }

    async fn load_from_storage(&self, storage_path: String) -> Result<Session, SymbolError> {
        let content = tokio::fs::read_to_string(storage_path.to_owned())
            .await
            .map_err(|e| SymbolError::IOError(e))?;

        let session: Session = serde_json::from_str(&content).expect(&format!(
            "converting to session from json is okay: {storage_path}"
        ));
        Ok(session)
    }

    async fn save_to_storage(&self, session: &Session) -> Result<(), SymbolError> {
        let serialized = serde_json::to_string(session).unwrap();
        let mut file = tokio::fs::File::create(session.storage_path())
            .await
            .map_err(|e| SymbolError::IOError(e))?;
        file.write_all(serialized.as_bytes())
            .await
            .map_err(|e| SymbolError::IOError(e))?;
        Ok(())
    }
}
