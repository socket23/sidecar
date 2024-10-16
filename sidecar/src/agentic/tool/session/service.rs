//! Creates the service which handles saving the session and extending it

use std::sync::Arc;

use tokio::io::AsyncWriteExt;

use crate::{
    agentic::symbol::{
        errors::SymbolError, events::message_event::SymbolEventMessageProperties,
        manager::SymbolManager, tool_box::ToolBox,
    },
    repo::types::RepoRef,
    user_context::types::UserContext,
};

use super::session::{AideAgentMode, Session};

/// The session service which takes care of creating the session and manages the storage
pub struct SessionService {
    tool_box: Arc<ToolBox>,
    _symbol_manager: Arc<SymbolManager>,
}

impl SessionService {
    pub fn new(tool_box: Arc<ToolBox>, symbol_manager: Arc<SymbolManager>) -> Self {
        Self {
            tool_box,
            _symbol_manager: symbol_manager,
        }
    }

    fn create_new_session(
        &self,
        session_id: String,
        project_labels: Vec<String>,
        repo_ref: RepoRef,
        storage_path: String,
    ) -> Session {
        Session::new(session_id, project_labels, repo_ref, storage_path)
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
        message_properties: SymbolEventMessageProperties,
    ) -> Result<(), SymbolError> {
        println!("session_service::human_message::start");
        let mut session = if let Ok(session) = self.load_from_storage(storage_path.to_owned()).await
        {
            session
        } else {
            self.create_new_session(
                session_id,
                project_labels.to_vec(),
                repo_ref.clone(),
                storage_path,
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

        println!("session_service::reply_to_last_exchange");

        // now react to the last message
        session = session
            .reply_to_last_exchange(agent_mode, self.tool_box.clone(), message_properties)
            .await?;

        // save the session to the disk
        self.save_to_storage(&session).await?;
        Ok(())
    }

    async fn load_from_storage(&self, storage_path: String) -> Result<Session, SymbolError> {
        let content = tokio::fs::read_to_string(storage_path)
            .await
            .map_err(|e| SymbolError::IOError(e))?;

        let session: Session =
            serde_json::from_str(&content).expect("converting to session from json is okay");
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
