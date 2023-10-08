use std::sync::Arc;

use crate::{application::application::Application, repo::types::RepoRef};

use super::llm_funcs::LlmClient;

#[derive(Clone)]
pub struct ConversationMessage {
    id: uuid::Uuid,
    // The query which the user has asked
    query: String,
    // The steps which the agent has taken up until now
    steps_taken: Vec<AgentStep>,
    // The state of the agent
    agent_state: AgentState,
    // The action which the agent is going to take
    agent_action: AgentAction,
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

impl ConversationMessage {
    pub fn search_message(id: uuid::Uuid, agent_state: AgentState, query: String) -> Self {
        Self {
            id,
            agent_action: AgentAction::Query(query.to_owned()),
            query,
            steps_taken: vec![],
            agent_state,
            file_paths: vec![],
            code_spans: vec![],
            user_selected_code_span: vec![],
            open_files: vec![],
        }
    }

    pub fn add_agent_step(&mut self, step: AgentStep) {
        self.steps_taken.push(step);
    }

    pub fn add_code_spans(&mut self, code_span: CodeSpan) {
        self.code_spans.push(code_span);
    }
}

#[derive(Clone)]
pub struct CodeSpan {
    file_path: String,
    pub alias: usize,
    pub start_line: u64,
    end_line: u64,
    data: String,
}

impl CodeSpan {
    pub fn new(
        file_path: String,
        alias: usize,
        start_line: u64,
        end_line: u64,
        data: String,
    ) -> Self {
        Self {
            file_path,
            alias,
            start_line,
            end_line,
            data,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.trim().is_empty()
    }
}

impl std::fmt::Display for CodeSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}\n{}", self.alias, self.file_path, self.data)
    }
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
pub enum AgentAction {
    Query(String),
    Path { query: String },
    Code { query: String },
    Proc { query: String, paths: Vec<String> },
    Answer { paths: Vec<String> },
}

#[derive(Clone)]
pub struct Agent {
    pub application: Application,
    reporef: RepoRef,
    session_id: uuid::Uuid,
    conversation_state: ConversationState,
    pub conversation_messages: Vec<ConversationMessage>,
    llm_client: Arc<LlmClient>,
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
    pub fn prepare_for_search(
        application: Application,
        reporef: RepoRef,
        session_id: uuid::Uuid,
        query: &str,
        llm_client: Arc<LlmClient>,
    ) -> Self {
        // We will take care of the search here, and use that for the next steps
        let conversation_message = ConversationMessage::search_message(
            uuid::Uuid::new_v4(),
            AgentState::Search,
            query.to_owned(),
        );
        let agent = Agent {
            application,
            reporef,
            session_id,
            conversation_state: ConversationState::Pending,
            conversation_messages: vec![conversation_message],
            llm_client,
        };
        agent
    }

    pub fn get_llm_client(&self) -> Arc<LlmClient> {
        self.llm_client.clone()
    }

    pub fn reporef(&self) -> &RepoRef {
        &self.reporef
    }

    pub fn get_last_conversation_message(&mut self) -> &mut ConversationMessage {
        // If we don't have a conversation message then, we will crash and burn
        // here
        self.conversation_messages
            .last_mut()
            .expect("There should be a conversation message")
    }

    fn paths(&self) -> impl Iterator<Item = &str> {
        self.conversation_messages
            .iter()
            .flat_map(|e| e.file_paths.iter())
            .map(String::as_str)
    }

    pub fn get_path_alias(&mut self, path: &str) -> usize {
        // This has to be stored a variable due to a Rust NLL bug:
        // https://github.com/rust-lang/rust/issues/51826
        let pos = self.paths().position(|p| p == path);
        if let Some(i) = pos {
            i
        } else {
            let i = self.paths().count();
            self.get_last_conversation_message()
                .file_paths
                .push(path.to_owned());
            i
        }
    }
}
