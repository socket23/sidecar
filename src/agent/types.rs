use std::sync::Arc;

use tiktoken_rs::ChatCompletionRequestMessage;
use tracing::{debug, info};

use crate::{
    agent::llm_funcs::llm::FunctionCall, application::application::Application,
    indexes::indexer::FileDocument, repo::types::RepoRef,
};

use super::{
    llm_funcs::{self, LlmClient},
    model, prompts,
};

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
    // The status of this conversation
    conversation_state: ConversationState,
    // Final answer which is going to get stored here
    answer: Option<String>,
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
            conversation_state: ConversationState::Started,
            answer: None,
        }
    }

    pub fn add_agent_step(&mut self, step: AgentStep) {
        self.steps_taken.push(step);
    }

    pub fn add_code_spans(&mut self, code_span: CodeSpan) {
        self.code_spans.push(code_span);
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn answer(&self) -> Option<String> {
        self.answer.clone()
    }

    pub fn set_answer(&mut self, answer: String) {
        self.answer = Some(answer);
    }
}

#[derive(Clone, Debug)]
pub struct CodeSpan {
    pub file_path: String,
    pub alias: usize,
    pub start_line: u64,
    pub end_line: u64,
    pub data: String,
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

impl AgentStep {
    pub fn get_response(&self) -> String {
        match self {
            AgentStep::Path { response, .. } => response.to_owned(),
            AgentStep::Code { response, .. } => response.to_owned(),
            AgentStep::Proc { response, .. } => response.to_owned(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentAction {
    Query(String),
    Path {
        query: String,
    },
    Code {
        query: String,
    },
    Proc {
        query: String,
        paths: Vec<usize>,
    },
    #[serde(rename = "none")]
    Answer {
        paths: Vec<usize>,
    },
}

impl AgentAction {
    pub fn from_gpt_response(call: &FunctionCall) -> anyhow::Result<Self> {
        let mut map = serde_json::Map::new();
        map.insert(call.name.clone(), serde_json::from_str(&call.arguments)?);

        Ok(serde_json::from_value(serde_json::Value::Object(map))?)
    }
}

#[derive(Clone)]
pub struct Agent {
    pub application: Application,
    pub reporef: RepoRef,
    pub session_id: uuid::Uuid,
    pub conversation_messages: Vec<ConversationMessage>,
    pub llm_client: Arc<LlmClient>,
    pub model: model::AnswerModel,
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

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.conversation_messages
            .iter()
            .flat_map(|e| e.file_paths.iter())
            .map(String::as_str)
    }

    pub async fn get_file_content(&self, path: &str) -> anyhow::Result<Option<FileDocument>> {
        debug!(%self.reporef, path, %self.session_id, "executing file search");
        let file_reader = self
            .application
            .indexes
            .file
            .get_by_path(path, &self.reporef)
            .await;
        file_reader
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

    /// The full history of messages, including intermediate function calls
    fn history(&self) -> anyhow::Result<Vec<llm_funcs::llm::Message>> {
        const ANSWER_MAX_HISTORY_SIZE: usize = 3;
        const FUNCTION_CALL_INSTRUCTION: &str = "Call a function. Do not answer";

        let history = self
            .conversation_messages
            .iter()
            .rev()
            .take(ANSWER_MAX_HISTORY_SIZE)
            .rev()
            .try_fold(Vec::new(), |mut acc, e| -> anyhow::Result<_> {
                let query = llm_funcs::llm::Message::user(e.query());

                let steps = e.steps_taken.iter().flat_map(|s| {
                    let (name, arguments) = match s {
                        AgentStep::Path { query, .. } => (
                            "path".to_owned(),
                            format!("{{\n \"query\": \"{query}\"\n}}"),
                        ),
                        AgentStep::Code { query, .. } => (
                            "code".to_owned(),
                            format!("{{\n \"query\": \"{query}\"\n}}"),
                        ),
                        AgentStep::Proc { query, paths, .. } => (
                            "proc".to_owned(),
                            format!(
                                "{{\n \"paths\": [{}],\n \"query\": \"{query}\"\n}}",
                                paths
                                    .iter()
                                    .map(|path| self
                                        .paths()
                                        .position(|p| p == path)
                                        .unwrap()
                                        .to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        ),
                    };

                    vec![
                        llm_funcs::llm::Message::function_call(FunctionCall {
                            name: name.clone(),
                            arguments,
                        }),
                        llm_funcs::llm::Message::function_return(name.to_owned(), s.get_response()),
                        llm_funcs::llm::Message::user(FUNCTION_CALL_INSTRUCTION),
                    ]
                });

                let answer = match e.answer() {
                    // NB: We intentionally discard the summary as it is redundant.
                    Some(answer) => Some(llm_funcs::llm::Message::function_return(
                        "none".to_owned(),
                        answer,
                    )),
                    None => None,
                };

                acc.extend(
                    std::iter::once(query)
                        .chain(vec![llm_funcs::llm::Message::user(
                            FUNCTION_CALL_INSTRUCTION,
                        )])
                        .chain(steps)
                        .chain(answer.into_iter()),
                );
                Ok(acc)
            })?;
        Ok(history)
    }

    pub fn code_spans(&self) -> Vec<CodeSpan> {
        self.conversation_messages
            .iter()
            .flat_map(|e| e.code_spans.clone())
            .collect()
    }

    pub async fn iterate(&mut self, action: AgentAction) -> anyhow::Result<Option<AgentAction>> {
        // Now we will go about iterating over the action and figure out what the
        // next best action should be
        match action {
            AgentAction::Answer { paths } => {
                // here we can finally answer after we do some merging on the spans
                // and also look at the history to provide more context
                let answer = self.answer(paths.as_slice()).await?;
                info!(%self.session_id, "conversation finished");
                info!(%self.session_id, answer, "answer");
                return Ok(None);
            }
            AgentAction::Code { query } => self.code_search(&query).await?,
            AgentAction::Path { query } => self.path_search(&query).await?,
            AgentAction::Proc { query, paths } => {
                self.process_files(&query, paths.as_slice()).await?
            }
            AgentAction::Query(query) => {
                // just log here for now
                query.clone()
            }
        };

        let functions = serde_json::from_value::<Vec<llm_funcs::llm::Function>>(
            prompts::functions(self.paths().next().is_some()), // Only add proc if there are paths in context
        )
        .unwrap();

        let mut history = vec![llm_funcs::llm::Message::system(&prompts::system(
            self.paths(),
        ))];
        history.extend(self.history()?);

        let trimmed_history = trim_history(history.clone(), self.model.clone())?;

        let response = self
            .get_llm_client()
            .stream_response(
                llm_funcs::llm::OpenAIModel::get_model(self.model.model_name)?,
                trimmed_history,
                Some(functions),
                0.0,
                None,
            )
            .await?;

        dbg!(response);

        // Now that we will be here, we have to figure out how to get the function
        // which needs to be run from the choices we are getting from the llm
        // response

        // To get the next thing here, we will have to look at the history
        // and them trim it down
        Ok(None)
    }
}

fn trim_history(
    mut history: Vec<llm_funcs::llm::Message>,
    model: model::AnswerModel,
) -> anyhow::Result<Vec<llm_funcs::llm::Message>> {
    const HIDDEN: &str = "[HIDDEN]";

    let mut tiktoken_msgs: Vec<ChatCompletionRequestMessage> =
        history.iter().map(|m| m.into()).collect::<Vec<_>>();

    while tiktoken_rs::get_chat_completion_max_tokens(model.tokenizer, &tiktoken_msgs)?
        < model.history_tokens_limit
    {
        let _ = history
            .iter_mut()
            .zip(tiktoken_msgs.iter_mut())
            .position(|(m, tm)| match m {
                llm_funcs::llm::Message::PlainText {
                    role,
                    ref mut content,
                } => {
                    if role == &llm_funcs::llm::Role::Assistant && content != HIDDEN {
                        *content = HIDDEN.into();
                        tm.content = Some(HIDDEN.into());
                        true
                    } else {
                        false
                    }
                }
                llm_funcs::llm::Message::FunctionReturn {
                    role: _,
                    name: _,
                    ref mut content,
                } if content != HIDDEN => {
                    *content = HIDDEN.into();
                    tm.content = Some(HIDDEN.into());
                    true
                }
                _ => false,
            })
            .ok_or_else(|| anyhow::anyhow!("could not find message to trim"))?;
    }

    Ok(history)
}
