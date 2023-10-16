use std::sync::Arc;

use rake::Rake;
use tiktoken_rs::ChatCompletionRequestMessage;
use tokio::sync::mpsc::Sender;
use tracing::{debug, info};

use crate::{
    agent::llm_funcs::llm::FunctionCall, application::application::Application, db::sqlite::SqlDb,
    indexes::indexer::FileDocument, repo::types::RepoRef,
};

use super::{
    llm_funcs::{self, LlmClient},
    model, prompts,
    search::stop_words,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Answer {
    // This is the answer up-until now
    pub answer_up_until_now: String,
    // This is the delta between the previously sent answer and the current one
    // when streaming we often times want to send the delta so we can show
    // the output in a streaming fashion
    pub delta: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationMessage {
    message_id: uuid::Uuid,
    // We also want to store the session id here so we can load it and save it
    session_id: uuid::Uuid,
    // The query which the user has asked
    query: String,
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
    // The status of this conversation
    conversation_state: ConversationState,
    // Final answer which is going to get stored here
    answer: Option<Answer>,
    // Last updated
    last_updated: u64,
    // Created at
    created_at: u64,
}

impl ConversationMessage {
    pub fn search_message(session_id: uuid::Uuid, agent_state: AgentState, query: String) -> Self {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            message_id: uuid::Uuid::new_v4(),
            session_id,
            query,
            steps_taken: vec![],
            agent_state,
            file_paths: vec![],
            code_spans: vec![],
            user_selected_code_span: vec![],
            open_files: vec![],
            conversation_state: ConversationState::Started,
            answer: None,
            created_at: current_time,
            last_updated: current_time,
        }
    }

    pub fn semantic_search(session_id: uuid::Uuid, agent_state: AgentState, query: String) -> Self {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            message_id: uuid::Uuid::new_v4(),
            session_id,
            query,
            steps_taken: vec![],
            agent_state,
            file_paths: vec![],
            code_spans: vec![],
            user_selected_code_span: vec![],
            open_files: vec![],
            conversation_state: ConversationState::Started,
            answer: None,
            created_at: current_time,
            last_updated: current_time,
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

    pub fn answer(&self) -> Option<Answer> {
        self.answer.clone()
    }

    pub fn set_answer(&mut self, answer: String) {
        // It's important that we mark the conversation as finished
        self.conversation_state = ConversationState::Finished;
        self.answer = Some(Answer {
            answer_up_until_now: answer,
            delta: None,
        });
    }

    pub fn answer_update(session_id: uuid::Uuid, answer_update: Answer) -> Self {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            message_id: uuid::Uuid::new_v4(),
            session_id,
            query: String::new(),
            steps_taken: vec![],
            agent_state: AgentState::Finish,
            file_paths: vec![],
            code_spans: vec![],
            user_selected_code_span: vec![],
            open_files: vec![],
            conversation_state: ConversationState::StreamingAnswer,
            answer: Some(answer_update),
            created_at: current_time,
            last_updated: current_time,
        }
    }

    pub async fn save_to_db(&self, db: SqlDb) -> anyhow::Result<()> {
        debug!(%self.session_id, %self.message_id, "saving conversation message to db");
        let mut tx = db.begin().await?;
        let message_id = self.message_id.to_string();
        let query = self.query.to_owned();
        let answer = self
            .answer
            .as_ref()
            .map(|answer| answer.answer_up_until_now.to_owned());
        let created_at = self.created_at as i64;
        let last_updated = self.last_updated as i64;
        let session_id = self.session_id.to_string();
        let steps_taken = serde_json::to_string(&self.steps_taken)?;
        let agent_state = serde_json::to_string(&self.agent_state)?;
        let file_paths = serde_json::to_string(&self.file_paths)?;
        let code_spans = serde_json::to_string(&self.code_spans)?;
        let user_selected_code_span = serde_json::to_string(&self.user_selected_code_span)?;
        let open_files = serde_json::to_string(&self.open_files)?;
        let conversation_state = serde_json::to_string(&self.conversation_state)?;
        sqlx::query! {
            "INSERT INTO agent_conversation_message \
            (message_id, query, answer, created_at, last_updated, session_id, steps_taken, agent_state, file_paths, code_spans, user_selected_code_span, open_files, conversation_state) \
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            message_id,
            query,
            answer,
            created_at,
            last_updated,
            session_id,
            steps_taken,
            agent_state,
            file_paths,
            code_spans,
            user_selected_code_span,
            open_files,
            conversation_state,
        }.execute(&mut *tx).await?;
        let _ = tx.commit().await?;
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CodeSpan {
    pub file_path: String,
    pub alias: usize,
    pub start_line: u64,
    pub end_line: u64,
    pub data: String,
    pub score: Option<f32>,
}

impl CodeSpan {
    pub fn new(
        file_path: String,
        alias: usize,
        start_line: u64,
        end_line: u64,
        data: String,
        score: Option<f32>,
    ) -> Self {
        Self {
            file_path,
            alias,
            start_line,
            end_line,
            data,
            score,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.trim().is_empty()
    }

    pub fn get_unique_key(&self) -> String {
        format!("{}:{}-{}", self.file_path, self.start_line, self.end_line)
    }
}

impl std::fmt::Display for CodeSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}\n{}", self.alias, self.file_path, self.data)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum AgentStep {
    Path {
        query: String,
        response: String,
        paths: Vec<String>,
    },
    Code {
        query: String,
        response: String,
        code_snippets: Vec<CodeSpan>,
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
        map.insert(
            call.name
                .as_ref()
                .expect("function_name to be present in function_call")
                .to_owned(),
            serde_json::from_str(&call.arguments)?,
        );

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
    pub sql_db: SqlDb,
    pub sender: Sender<ConversationMessage>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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
    // If we are performing semantic search, we should be over here
    SemanticSearch,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum ConversationState {
    Pending,
    Started,
    StreamingAnswer,
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
        const FUNCTION_CALL_INSTRUCTION: &str = "CALL A FUNCTION!. Do not answer";

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
                            name: Some(name.to_owned()),
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
                        answer.answer_up_until_now,
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

    pub async fn iterate(
        &mut self,
        action: AgentAction,
        answer_sender: tokio::sync::mpsc::UnboundedSender<Answer>,
    ) -> anyhow::Result<Option<AgentAction>> {
        // Now we will go about iterating over the action and figure out what the
        // next best action should be
        match action {
            AgentAction::Answer { paths } => {
                // here we can finally answer after we do some merging on the spans
                // and also look at the history to provide more context
                let answer = self.answer(paths.as_slice(), answer_sender).await?;
                info!(%self.session_id, "conversation finished");
                info!(%self.session_id, answer, "answer");
                // We should make it an atomic operation where whenever we update
                // the conversation, we send an update on the stream and also
                // save it to the db
                if let Some(last_conversation) = self.conversation_messages.last() {
                    // save the conversation to the DB
                    let _ = last_conversation.save_to_db(self.sql_db.clone()).await;
                    // send it over the sender
                    let _ = self.sender.send(last_conversation.clone()).await;
                }
                return Ok(None);
            }
            AgentAction::Code { query } => self.code_search(&query).await?,
            AgentAction::Path { query } => self.path_search(&query).await?,
            AgentAction::Proc { query, paths } => {
                self.process_files(&query, paths.as_slice()).await?
            }
            AgentAction::Query(query) => {
                // just log here for now
                let cloned_query = query.clone();
                // we want to do a search anyways here using the keywords, so we
                // have some kind of context
                // Always make a code search for the user query on the first exchange
                if self.conversation_messages.len() == 1 {
                    // Extract keywords from the query
                    let keywords = {
                        let sw = stop_words();
                        let r = Rake::new(sw.clone());
                        let keywords = r.run(&cloned_query);

                        if keywords.is_empty() {
                            cloned_query.to_owned()
                        } else {
                            keywords
                                .iter()
                                .map(|k| k.keyword.clone())
                                .collect::<Vec<_>>()
                                .join(" ")
                        }
                    };

                    debug!(%self.session_id, %keywords, "extracted keywords from query");

                    let response = self.code_search(&keywords).await;
                    debug!(?response, "code search response");
                }
                query.clone()
            }
        };

        // We also retroactively save the last conversation to the database
        if let Some(last_conversation) = self.conversation_messages.last() {
            // save the conversation to the DB
            let _ = last_conversation.save_to_db(self.sql_db.clone()).await;
            // send it over the sender
            let _ = self.sender.send(last_conversation.clone()).await;
        }

        let functions = serde_json::from_value::<Vec<llm_funcs::llm::Function>>(
            prompts::functions(self.paths().next().is_some()), // Only add proc if there are paths in context
        )
        .unwrap();

        let mut history = vec![llm_funcs::llm::Message::system(&prompts::system_search(
            self.paths(),
        ))];
        history.extend(self.history()?);

        let trimmed_history = trim_history(history.clone(), self.model.clone())?;

        let response = self
            .get_llm_client()
            .stream_function_call(
                llm_funcs::llm::OpenAIModel::get_model(self.model.model_name)?,
                trimmed_history,
                functions,
                0.0,
                None,
            )
            .await?;

        if let Some(response) = response {
            AgentAction::from_gpt_response(&response).map(|response| Some(response))
        } else {
            Ok(None)
        }
    }

    pub async fn semantic_search(&mut self) -> anyhow::Result<Vec<CodeSpan>> {
        let model = llm_funcs::llm::OpenAIModel::get_model(self.model.model_name)?;
        // Get or create the history from the conversation before so we can better
        // select
        let mut history = vec![llm_funcs::llm::Message::system(
            &prompts::system_sematic_search(self.paths()),
        )];
        history.extend(self.history()?);
        let trimmed_history = trim_history(history.clone(), self.model.clone())?;

        let path_search_function =
            serde_json::from_value::<llm_funcs::llm::Function>(prompts::path_function()).unwrap();
        let response = self
            .get_llm_client()
            .stream_function_call(
                model,
                trimmed_history,
                vec![path_search_function],
                0.0,
                None,
            )
            .await?;
        let path_function_call = if let Some(response) = response {
            AgentAction::from_gpt_response(&response).map(|response| Some(response))
        } else {
            Ok(None)
        };
        // At this point when we perform the search using the path function first
        // so we can do lexical search first
        if let Ok(Some(AgentAction::Path { query })) = path_function_call {
            self.path_search(&query).await.expect("path search to work");
        }

        // Now we go here and perform a semantic search with the paths we have
        let semantic_search_function =
            serde_json::from_value::<llm_funcs::llm::Function>(prompts::code_function()).unwrap();

        let mut history = vec![llm_funcs::llm::Message::system(
            &prompts::system_sematic_search(self.paths()),
        )];
        history.extend(self.history()?);

        let trimmed_history = trim_history(history.clone(), self.model.clone())?;

        let semantic_search_query = self
            .get_llm_client()
            .stream_function_call(
                llm_funcs::llm::OpenAIModel::get_model(self.model.model_name)?,
                trimmed_history,
                vec![semantic_search_function],
                0.0,
                None,
            )
            .await?;
        let semantic_search_query = if let Some(response) = semantic_search_query {
            AgentAction::from_gpt_response(&response).map(|response| Some(response))
        } else {
            Ok(None)
        };

        if let Ok(Some(AgentAction::Code { query })) = semantic_search_query {
            self.code_search(&query).await.expect("code search to work");
        }

        // At this point we have the code snippets, and we can return this back
        // from this function
        Ok(self.get_last_conversation_message().code_spans.to_vec())
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

impl Drop for Agent {
    fn drop(&mut self) {
        // Now we will try to save all the conversations to the database
        let db = self.sql_db.clone();
        let conversation_messages = self.conversation_messages.to_vec();
        // This will save all the pending conversations to the database
        tokio::spawn(async move {
            use futures::StreamExt;
            futures::stream::iter(conversation_messages)
                .map(|conversation| (conversation, db.clone()))
                .map(|(conversation, db)| async move { conversation.save_to_db(db.clone()).await })
                .collect::<Vec<_>>()
                .await;
        });
    }
}
