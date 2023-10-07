use crate::{application::application::Application, repo::types::RepoRef};

#[derive(Clone)]
pub struct Agent {
    application: Application,
    reporef: RepoRef,
    session_id: String,
    conversation_state: ConversationState,
}

#[derive(Clone)]
pub enum ConversationState {
    Pending,
    Started,
    Finished,
}
