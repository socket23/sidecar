use crate::{application::application::Application, repo::types::RepoRef};

#[derive(Clone)]
pub struct ConversationMessage {
    id: uuid::Uuid,
    // The steps which the agent has taken up until now
    steps_taken: Vec<AgentStep>,
    // The state of the agent
    agent_state: AgentState,
    // The file paths we are interested in, can be populated via search or after
    // asking for more context
    file_paths: Vec<String>,
    // The span which we found after performing search
    code_spans: Vec<CodeSpan>,
    // The span which user has selected and added to the context
    user_selected_code_span: Vec<CodeSpan>,
    // The files which are open in the editor
    open_files: Vec<String>,
}

#[derive(Clone)]
pub struct CodeSpan {
    file_path: String,
    start_line: usize,
    end_line: usize,
    data: String,
}

#[derive(Clone)]
pub enum AgentStep {
    Path {
        query: String,
        response: String,
    },
    Code {
        query: String,
        response: String,
    },
    Proc {
        query: String,
        paths: Vec<String>,
        response: String,
    },
}

#[derive(Clone)]
pub struct Agent {
    application: Application,
    reporef: RepoRef,
    session_id: String,
    conversation_state: ConversationState,
    conversation_messages: Vec<ConversationMessage>,
}

#[derive(Clone)]
pub enum AgentState {
    // We will end up doing a search
    Search,
    // Plan out what the changes are required for the agent to do
    Plan,
    // Explain to the user what the code is going to do
    Explain,
    // The code editing which needs to be done
    CodeEdit,
    // Fix the linters and everything else over here
    FixSignals,
    // We finish up the work of the agent
    Finish,
}

#[derive(Clone)]
pub enum ConversationState {
    Pending,
    Started,
    Finished,
}

impl Agent {
    pub fn search(&self, query: &str) {}
}
