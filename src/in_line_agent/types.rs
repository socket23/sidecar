use std::sync::Arc;
use tokio::sync::mpsc::{Sender, UnboundedSender};

use crate::{
    agent::{
        llm_funcs::{self, LlmClient},
        model,
    },
    application::application::Application,
    db::sqlite::SqlDb,
    repo::types::RepoRef,
};

use super::prompts;

#[derive(Debug, Clone)]
pub struct InLineAgentAnswer {
    pub answer_un_until_now: String,
    pub delta: Option<String>,
}

#[derive(Debug, Clone)]
pub enum InLineAgentAction {
    // Add code to an already existing codebase
    Code,
    // Add documentation comment for this symbol
    Doc,
    // Refactors the selected code based on requirements provided by the user
    Edit,
    // Generate unit tests for the selected code
    Tests,
    // Propose a fix for the problems in the selected code
    Fix,
    // Explain how the selected code snippet works
    Explain,
    // Intent of this command is unclear or is not related to the information technologies
    Unknown,
    // decide the next action the agent should take, this is the first state always
    DecideAction { query: String },
}

impl InLineAgentAction {
    pub fn from_gpt_response(response: &str) -> anyhow::Result<Self> {
        match response {
            "code" => Ok(Self::Code),
            "doc" => Ok(Self::Doc),
            "edit" => Ok(Self::Edit),
            "tests" => Ok(Self::Tests),
            "fix" => Ok(Self::Fix),
            "explain" => Ok(Self::Explain),
            "unknown" => Ok(Self::Unknown),
            _ => anyhow::bail!("unknown response from gpt: {}", response),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InLineAgentMessage {}

impl InLineAgentMessage {
    pub fn answer_update(_session_id: uuid::Uuid, _answer_update: InLineAgentAnswer) -> Self {
        Self {}
    }
}

/// We have an inline agent which takes care of questions which are asked in-line
/// this agent behaves a bit different than the general agent which we provide
/// as a chat, so there are different states and other things which this agent
/// takes care of
#[derive(Clone)]
pub struct InLineAgent {
    pub application: Application,
    pub repo_ref: RepoRef,
    pub session_id: uuid::Uuid,
    pub inline_agent_messages: Vec<InLineAgentMessage>,
    pub llm_client: Arc<LlmClient>,
    pub model: model::AnswerModel,
    pub sql_db: SqlDb,
    pub sender: Sender<InLineAgentMessage>,
}

impl InLineAgent {
    pub fn new(
        application: Application,
        repo_ref: RepoRef,
        sql_db: SqlDb,
        llm_client: Arc<LlmClient>,
        sender: Sender<InLineAgentMessage>,
    ) -> Self {
        Self {
            application,
            repo_ref,
            session_id: uuid::Uuid::new_v4(),
            inline_agent_messages: vec![],
            llm_client,
            model: model::GPT_3_5_TURBO_16K,
            sql_db,
            sender,
        }
    }

    fn get_llm_client(&self) -> Arc<LlmClient> {
        self.llm_client.clone()
    }

    pub async fn iterate(
        &mut self,
        action: InLineAgentAction,
        answer_sender: UnboundedSender<InLineAgentAnswer>,
    ) -> anyhow::Result<Option<InLineAgentAction>> {
        match action {
            InLineAgentAction::DecideAction { query } => {
                // Decide the action we are want to take here
                let next_action = self.decide_action(&query).await?;
                Ok(Some(next_action))
            }
            _ => {
                unimplemented!();
            }
        }
    }

    async fn decide_action(&self, query: &str) -> anyhow::Result<InLineAgentAction> {
        let model = llm_funcs::llm::OpenAIModel::get_model(self.model.model_name)?;
        let system_prompt = prompts::decide_function_to_use(query);
        let messages = vec![llm_funcs::llm::Message::system(&system_prompt)];
        let response = self
            .get_llm_client()
            .response(model, messages, None, 0.0, None)
            .await?;
        InLineAgentAction::from_gpt_response(&response)
    }
}
