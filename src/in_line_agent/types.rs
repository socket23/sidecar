use futures::stream;
use futures::StreamExt;
use regex::Regex;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, UnboundedSender};

use crate::chunking::text_document::Range;
use crate::chunking::types::FunctionInformation;
use crate::chunking::types::FunctionNodeType;
use crate::in_line_agent::context_parsing::generate_context_for_range;
use crate::in_line_agent::context_parsing::ContextParserInLineEdit;
use crate::in_line_agent::context_parsing::EditExpandedSelectionRange;
use crate::webserver::agent::Position;
use crate::{
    agent::{
        llm_funcs::{self, llm::Message, LlmClient},
        model,
    },
    application::application::Application,
    chunking::{
        editor_parsing::EditorParsing,
        text_document::{DocumentSymbol, TextDocument},
    },
    db::sqlite::SqlDb,
    repo::types::RepoRef,
    webserver::in_line_agent::ProcessInEditorRequest,
};

use super::context_parsing::generate_selection_context_for_fix;
use super::context_parsing::ContextWindowTracker;
use super::context_parsing::SelectionContext;
use super::context_parsing::SelectionWithOutlines;
use super::prompts;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InLineAgentSelectionData {
    has_content: bool,
    first_line_index: i64,
    last_line_index: i64,
    lines: Vec<String>,
}

impl InLineAgentSelectionData {
    pub fn new(
        has_content: bool,
        first_line_index: i64,
        last_line_index: i64,
        lines: Vec<String>,
    ) -> Self {
        Self {
            has_content,
            first_line_index,
            last_line_index,
            lines,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextSelection {
    above: InLineAgentSelectionData,
    range: InLineAgentSelectionData,
    below: InLineAgentSelectionData,
}

impl ContextSelection {
    pub fn new(
        above: InLineAgentSelectionData,
        range: InLineAgentSelectionData,
        below: InLineAgentSelectionData,
    ) -> Self {
        Self {
            above,
            range,
            below,
        }
    }

    pub fn from_selection_context(selection_context: SelectionContext) -> Self {
        Self {
            above: selection_context.above.to_agent_selection_data(),
            range: selection_context.range.to_agent_selection_data(),
            below: selection_context.below.to_agent_selection_data(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InLineAgentAnswer {
    pub answer_up_until_now: String,
    pub delta: Option<String>,
    pub state: MessageState,
    // We also send the document symbol in question along the wire
    pub document_symbol: Option<DocumentSymbol>,
    pub context_selection: Option<ContextSelection>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MessageState {
    Pending,
    Started,
    StreamingAnswer,
    Finished,
    Errored,
}

impl Default for MessageState {
    fn default() -> Self {
        MessageState::StreamingAnswer
    }
}

impl InLineAgentAction {
    pub fn from_gpt_response(response: &str) -> anyhow::Result<Self> {
        match response.trim() {
            "code" => Ok(Self::Code),
            "doc" => Ok(Self::Doc),
            "edit" => Ok(Self::Edit),
            "tests" => Ok(Self::Tests),
            "fix" => Ok(Self::Fix),
            "explain" => Ok(Self::Explain),
            "unknown" => Ok(Self::Unknown),
            _ => Ok(Self::Unknown),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InLineAgentMessage {
    message_id: uuid::Uuid,
    session_id: uuid::Uuid,
    query: String,
    steps_taken: Vec<InLineAgentAction>,
    message_state: MessageState,
    answer: Option<InLineAgentAnswer>,
    last_updated: u64,
    created_at: u64,
}

impl InLineAgentMessage {
    pub fn decide_action(
        session_id: uuid::Uuid,
        query: String,
        agent_state: InLineAgentAction,
    ) -> Self {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            message_id: uuid::Uuid::new_v4(),
            session_id,
            query,
            steps_taken: vec![agent_state],
            message_state: MessageState::Started,
            answer: None,
            last_updated: current_time,
            created_at: current_time,
        }
    }

    pub fn answer_update(session_id: uuid::Uuid, answer_update: InLineAgentAnswer) -> Self {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            message_id: uuid::Uuid::new_v4(),
            session_id,
            query: String::new(),
            steps_taken: vec![],
            message_state: MessageState::StreamingAnswer,
            answer: Some(answer_update),
            last_updated: current_time,
            created_at: current_time,
        }
    }

    pub fn start_message(session_id: uuid::Uuid, query: String) -> Self {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            message_id: uuid::Uuid::new_v4(),
            session_id,
            query,
            steps_taken: vec![],
            message_state: MessageState::Pending,
            answer: None,
            last_updated: current_time,
            created_at: current_time,
        }
    }

    pub fn add_agent_action(&mut self, agent_action: InLineAgentAction) {
        self.steps_taken.push(agent_action);
    }
}

/// We have an inline agent which takes care of questions which are asked in-line
/// this agent behaves a bit different than the general agent which we provide
/// as a chat, so there are different states and other things which this agent
/// takes care of
#[derive(Clone)]
pub struct InLineAgent {
    application: Application,
    repo_ref: RepoRef,
    session_id: uuid::Uuid,
    inline_agent_messages: Vec<InLineAgentMessage>,
    llm_client: Arc<LlmClient>,
    model: model::AnswerModel,
    sql_db: SqlDb,
    editor_parsing: EditorParsing,
    // TODO(skcd): Break this out and don't use cross crate dependency like this
    editor_request: ProcessInEditorRequest,
    sender: Sender<InLineAgentMessage>,
}

impl InLineAgent {
    pub fn new(
        application: Application,
        repo_ref: RepoRef,
        sql_db: SqlDb,
        llm_client: Arc<LlmClient>,
        editor_parsing: EditorParsing,
        editor_request: ProcessInEditorRequest,
        messages: Vec<InLineAgentMessage>,
        sender: Sender<InLineAgentMessage>,
    ) -> Self {
        Self {
            application,
            repo_ref,
            session_id: uuid::Uuid::new_v4(),
            inline_agent_messages: messages,
            llm_client,
            model: model::GPT_3_5_TURBO_16K,
            sql_db,
            sender,
            editor_request,
            editor_parsing,
        }
    }

    fn get_llm_client(&self) -> Arc<LlmClient> {
        self.llm_client.clone()
    }

    fn last_agent_message(&self) -> Option<&InLineAgentMessage> {
        self.inline_agent_messages.last()
    }

    fn get_last_agent_message(&mut self) -> &mut InLineAgentMessage {
        self.inline_agent_messages
            .last_mut()
            .expect("There should always be a agent message")
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

                // Send it to the answer sender so we can show it on the frontend
                if let Some(last_exchange) = self.last_agent_message() {
                    self.sender.send(last_exchange.clone()).await?;
                }
                return Ok(Some(next_action));
            }
            InLineAgentAction::Doc => {
                // If we are going to document something, then we go into
                // this flow here
                // First we update our state that we are now going to generate documentation
                let last_exchange;
                {
                    let last_exchange_ref = self.get_last_agent_message();
                    last_exchange_ref.add_agent_action(InLineAgentAction::Doc);
                    last_exchange = last_exchange_ref.clone();
                }
                // and send it over the sender too
                {
                    self.sender.send(last_exchange.clone()).await?;
                }
                // and then we start generating the documentation
                self.generate_documentation(answer_sender).await?;
                return Ok(None);
            }
            // For both the edit and the code we use the same functionality right
            // now, we will give them separate commands later on
            InLineAgentAction::Edit | InLineAgentAction::Code => {
                // First we update our state here
                let last_exchange;
                {
                    let last_exchange_ref = self.get_last_agent_message();
                    last_exchange_ref.add_agent_action(InLineAgentAction::Edit);
                    last_exchange = last_exchange_ref.clone();
                }
                // send it over the wire
                {
                    self.sender.send(last_exchange.clone()).await?;
                }
                // and then we start generating the edit and send it over
                self.process_edit(answer_sender).await?;
                return Ok(None);
            }
            InLineAgentAction::Fix => {
                let last_exchange;
                {
                    let last_exchange_ref = self.get_last_agent_message();
                    last_exchange_ref.add_agent_action(InLineAgentAction::Fix);
                    last_exchange = last_exchange_ref.clone();
                }
                // send it over the wire
                {
                    self.sender.send(last_exchange.clone()).await?;
                }
                // and then we start generating the fix and send it over
                self.process_fix(answer_sender).await?;
                return Ok(None);
            }
            _ => {
                self.apologise_message().await?;
                return Ok(None);
            }
        }
    }

    async fn decide_action(&mut self, query: &str) -> anyhow::Result<InLineAgentAction> {
        let model = llm_funcs::llm::OpenAIModel::get_model(self.model.model_name)?;
        let system_prompt = prompts::decide_function_to_use(query);
        let messages = vec![llm_funcs::llm::Message::system(&system_prompt)];
        let response = self
            .get_llm_client()
            .response(model, messages, None, 0.0, None)
            .await?;
        let last_exchange = self.get_last_agent_message();
        // We add that we took a action to decide what we should do next
        last_exchange.add_agent_action(InLineAgentAction::DecideAction {
            query: query.to_owned(),
        });
        InLineAgentAction::from_gpt_response(&response)
    }

    async fn generate_documentation(
        &mut self,
        answer_sender: UnboundedSender<InLineAgentAnswer>,
    ) -> anyhow::Result<()> {
        // Now we get to the documentation generation loop, here we want to
        // first figure out what the context of the document is which we want
        // to generate the documentation for
        let source_str = self.editor_request.text_document_web.text.to_owned();
        let language = self.editor_request.text_document_web.language.to_owned();
        let relative_path = self
            .editor_request
            .text_document_web
            .relative_path
            .to_owned();
        let fs_file_path = self
            .editor_request
            .text_document_web
            .fs_file_path
            .to_owned();
        let start_position = self
            .editor_request
            .snippet_information
            .start_position
            .clone();
        let end_position = self.editor_request.snippet_information.end_position.clone();
        let request = self.editor_request.query.to_owned();
        let document_nodes = self.editor_parsing.get_documentation_node_for_range(
            &source_str,
            &language,
            &relative_path,
            &fs_file_path,
            &start_position,
            &end_position,
            &self.repo_ref,
        );
        let last_exchange = self.get_last_agent_message();
        if document_nodes.is_empty() {
            last_exchange.message_state = MessageState::Errored;
            answer_sender.send(InLineAgentAnswer {
                answer_up_until_now: "could not find documentation node".to_owned(),
                delta: Some("could not find documentation node".to_owned()),
                state: MessageState::Errored,
                document_symbol: None,
                context_selection: None,
            })?;
        } else {
            last_exchange.message_state = MessageState::StreamingAnswer;
            let messages_list = self.messages_for_documentation_generation(
                document_nodes,
                &language,
                &fs_file_path,
                &request,
            );
            let self_ = &*self;
            stream::iter(messages_list)
                .map(|messages| (messages, answer_sender.clone()))
                .for_each(|((messages, document_symbol), answer_sender)| async move {
                    let (proxy_sender, _proxy_receiver) = tokio::sync::mpsc::unbounded_channel();
                    let answer = self_
                        .get_llm_client()
                        .stream_response_inline_agent(
                            llm_funcs::llm::OpenAIModel::get_model(&self_.model.model_name)
                                .expect("openai model getting to always work"),
                            messages.messages,
                            None,
                            0.2,
                            None,
                            proxy_sender,
                            Some(document_symbol.clone()),
                            None,
                        )
                        .await;
                    // we send the answer after we have generated the whole thing
                    // not in between as its not proactive updates
                    if let Ok(answer) = answer {
                        answer_sender
                            .send(InLineAgentAnswer {
                                answer_up_until_now: answer.to_owned(),
                                delta: Some(answer.to_owned()),
                                state: Default::default(),
                                document_symbol: Some(document_symbol.clone()),
                                context_selection: None,
                            })
                            .unwrap();
                    }
                })
                .await;
        }
        // here we can have a case where we didn't detect any documentation node
        // if that's the case we should just reply with not found
        Ok(())
    }

    async fn apologise_message(&mut self) -> anyhow::Result<()> {
        let last_exchange = self.get_last_agent_message();
        last_exchange.add_agent_action(InLineAgentAction::Unknown);
        Ok(())
    }

    async fn process_fix(
        &mut self,
        answer_sender: UnboundedSender<InLineAgentAnswer>,
    ) -> anyhow::Result<()> {
        let fixing_range_maybe = self.application.language_parsing.get_fix_range(
            self.editor_request.source_code(),
            self.editor_request.language(),
            &self.editor_request.snippet_information.to_range(),
            15,
        );
        let fixing_range =
            fixing_range_maybe.unwrap_or(self.editor_request.snippet_information.to_range());

        let split_lines = Regex::new(r"\r\n|\r|\n").unwrap();
        let source_lines: Vec<String> = split_lines
            .split(&self.editor_request.source_code())
            .map(|s| s.to_owned())
            .collect();
        let character_limit = 8000;
        let mut token_tracker = ContextWindowTracker::new(character_limit);
        // Now we try to generate the snippet information
        let selection_context = generate_selection_context_for_fix(
            <i64>::try_from(self.editor_request.line_count()).unwrap(),
            &fixing_range,
            &self.editor_request.snippet_information.to_range(),
            self.editor_request.language(),
            source_lines,
            self.editor_request.fs_file_path().to_owned(),
            &mut token_tracker,
        );
        let user_prompts = self.fix_generation_prompt(&selection_context);
        let related_prompts = self.fix_diagnostics_prompt();
        let mut prompts = vec![llm_funcs::llm::Message::system(
            &prompts::fix_system_prompt(&self.editor_request.language()),
        )];
        prompts.extend(
            user_prompts
                .into_iter()
                .map(|prompt| llm_funcs::llm::Message::user(&prompt)),
        );
        prompts.extend(
            related_prompts
                .into_iter()
                .map(|prompt| llm_funcs::llm::Message::user(&prompt)),
        );
        let last_exchange = self.get_last_agent_message();
        last_exchange.message_state = MessageState::StreamingAnswer;
        let document_symbol = {
            let response_range = fixing_range;
            DocumentSymbol::for_edit(
                response_range.start_position(),
                response_range.end_position(),
            )
        };
        let self_ = &*self;
        let selection_context = ContextSelection::from_selection_context(selection_context);
        let answer = self
            .get_llm_client()
            .stream_response_inline_agent(
                llm_funcs::llm::OpenAIModel::get_model(&self_.model.model_name)
                    .expect("openai model getting to always work"),
                prompts,
                None,
                0.2,
                None,
                answer_sender,
                Some(document_symbol),
                Some(selection_context.clone()),
            )
            .await;
        // Now we need to create the prompts for diagnostics
        // now we need to get the diagnostics prompts
        Ok(())
    }

    async fn process_edit(
        &mut self,
        answer_sender: UnboundedSender<InLineAgentAnswer>,
    ) -> anyhow::Result<()> {
        // Here we will try to process the edits
        // This is the current request selection range
        let selection_range = Range::new(
            self.editor_request.start_position(),
            self.editor_request.end_position(),
        );
        // Now we want to get the chunks properly
        // First we get the function blocks along with the ranges we know about
        // we get the function nodes here
        let function_nodes = self.editor_parsing.function_information_nodes(
            &self.editor_request.source_code(),
            &self.editor_request.language(),
        );
        // Now we need to get the nodes which are just function blocks
        let mut function_blocks: Vec<_> = function_nodes
            .iter()
            .filter_map(|function_node| {
                if function_node.r#type() == &FunctionNodeType::Function {
                    Some(function_node)
                } else {
                    None
                }
            })
            .collect();
        // Now we sort the function blocks based on how close they are to the start index
        // of the code selection
        // we sort the nodes in increasing order
        function_blocks.sort_by(|a, b| a.range().start_byte().cmp(&b.range().start_byte()));

        // Next we need to get the function bodies
        let mut function_bodies: Vec<_> = function_nodes
            .iter()
            .filter_map(|function_node| {
                if function_node.r#type() == &FunctionNodeType::Body {
                    Some(function_node)
                } else {
                    None
                }
            })
            .collect();
        // Here we are sorting it in increasing order of start byte
        function_bodies.sort_by(|a, b| a.range().start_byte().cmp(&b.range().start_byte()));

        let expanded_selection = FunctionInformation::get_expanded_selection_range(
            function_blocks.as_slice(),
            &selection_range,
        );

        let edit_expansion = EditExpandedSelectionRange::new(
            Range::guard_large_expansion(selection_range.clone(), expanded_selection.clone(), 30),
            expanded_selection.clone(),
            FunctionInformation::fold_function_blocks(
                function_bodies
                    .to_vec()
                    .into_iter()
                    .map(|x| x.clone())
                    .collect(),
            ),
        );

        // these are the missing variables I have to fill in,
        // lines count and the source lines
        let split_lines = Regex::new(r"\r\n|\r|\n").unwrap();
        let source_lines: Vec<String> = split_lines
            .split(&self.editor_request.source_code())
            .map(|s| s.to_owned())
            .collect();
        // generate the prompts for it and then send it over to the LLM
        let response = generate_context_for_range(
            &self.editor_request.source_code(),
            self.editor_request.line_count(),
            &selection_range,
            &expanded_selection,
            &edit_expansion.range_expanded_to_functions,
            &self.editor_request.language(),
            // TODO(skcd): Make this more variable going forward
            4000,
            source_lines,
            function_bodies.into_iter().map(|fnb| fnb.clone()).collect(),
            self.editor_request.fs_file_path().to_owned(),
        );

        let selection_context = response.to_context_selection();

        // We create a fake document symbol which we will use to replace the
        // range which is present in the context of the selection
        let document_symbol = {
            let response_range = response.selection_context.get_selection_range();
            DocumentSymbol::for_edit(
                response_range.start_position(),
                response_range.end_position(),
            )
        };

        // We create the prompts here
        let mut user_messages = self
            .edit_generation_prompt(self.editor_request.language(), response)
            .into_iter()
            .map(|message| llm_funcs::llm::Message::user(&message))
            .collect::<Vec<_>>();
        // we emphasize again that it needs to always put the //BEGIN and //END
        let user_query = &self.editor_request.query;
        let user_query = format!("{user_query}\nDo not forget to include the // BEGIN and // END markers in your generated code.");
        user_messages.push(llm_funcs::llm::Message::user(&user_query));

        let mut final_messages = vec![llm_funcs::llm::Message::system(
            &prompts::in_line_edit_system_prompt(self.editor_request.language()),
        )];
        final_messages.extend(user_messages);

        // Now that we have the user-messages we can send it over the wire
        let last_exchange = self.get_last_agent_message();
        last_exchange.message_state = MessageState::StreamingAnswer;
        let self_ = &*self;
        let answer = self
            .get_llm_client()
            .stream_response_inline_agent(
                llm_funcs::llm::OpenAIModel::get_model(&self_.model.model_name)
                    .expect("openai model getting to always work"),
                final_messages,
                None,
                0.2,
                None,
                answer_sender,
                Some(document_symbol),
                Some(selection_context.clone()),
            )
            .await;
        Ok(())
    }

    pub fn messages_for_documentation_generation(
        &mut self,
        document_symbols: Vec<DocumentSymbol>,
        language: &str,
        file_path: &str,
        query: &str,
    ) -> Vec<(llm_funcs::llm::Messages, DocumentSymbol)> {
        document_symbols
            .into_iter()
            .map(|document_symbol| {
                let system_message = llm_funcs::llm::Message::system(
                    &prompts::documentation_system_prompt(language, document_symbol.kind.is_some()),
                );
                // Here we want to generate the context for the prompt
                let code_selection_prompt = llm_funcs::llm::Message::user(
                    &self.document_symbol_prompt(&document_symbol, language, file_path),
                );
                let user_prompt = format!(
                    "{} {}",
                    self.document_symbol_metadata(&document_symbol, language,),
                    query,
                );
                let metadata_prompt = llm_funcs::llm::Message::user(&user_prompt);
                (
                    llm_funcs::llm::Messages {
                        messages: vec![system_message, code_selection_prompt, metadata_prompt],
                    },
                    document_symbol,
                )
            })
            .collect::<Vec<_>>()
    }

    fn document_symbol_prompt(
        &self,
        document_symbol: &DocumentSymbol,
        language: &str,
        file_path: &str,
    ) -> String {
        let code = &document_symbol.code;
        let prompt_string = format!(
            r#"I have the following code in the selection:
```{language}
// FILEPATH: {file_path}
{code}
```
"#
        );
        prompt_string
    }

    fn document_symbol_metadata(&self, document_symbol: &DocumentSymbol, language: &str) -> String {
        let comment_type = match language {
            "typescript" | "typescriptreact" => match document_symbol.kind {
                Some(_) => "a TSDoc comment".to_owned(),
                None => "TSDoc comment".to_owned(),
            },
            "javascript" | "javascriptreact" => match document_symbol.kind {
                Some(_) => "a JSDoc comment".to_owned(),
                None => "JSDoc comment".to_owned(),
            },
            "python" => "docstring".to_owned(),
            "rust" => "Rustdoc comment".to_owned(),
            _ => "documentation comment".to_owned(),
        };

        // Now we want to generate the document symbol metadata properly
        match &document_symbol.name {
            Some(name) => {
                format!("Please add {comment_type} for {name}.")
            }
            None => {
                format!("Please add {comment_type} for the selection.")
            }
        }
    }

    fn fix_diagnostics_prompt(&self) -> Vec<String> {
        if let Some(diagnostics_information) = &self.editor_request.diagnostics_information {
            let first_message = &diagnostics_information.first_message;
            let related_information = diagnostics_information
                .diagnostic_information
                .iter()
                .map(|diagnostic| {
                    let prompt_parts = diagnostic.prompt_parts.to_vec();
                    let code_blocks: Vec<String> = diagnostic
                        .related_information
                        .iter()
                        .map(|related_information| {
                            let new_range = self
                                .application
                                .language_parsing
                                .get_parent_range_for_selection(
                                    &related_information.text,
                                    &related_information.language,
                                    &related_information.range,
                                );
                            let source_code = related_information.text
                                [new_range.start_byte()..new_range.end_byte()]
                                .to_owned();
                            wrap_in_code_block("", &source_code)
                        })
                        .collect();
                    if diagnostic.related_information.is_empty() {
                        prompt_parts.join("\n")
                    } else {
                        let mut answer = vec![prompt_parts.join("\n")];
                        answer.push("This diagnostic has some related code:".to_owned());
                        answer.extend(code_blocks.into_iter());
                        answer.join("\n")
                    }
                })
                .collect::<Vec<_>>();
            {
                vec![format!(
                    "{}\n{}",
                    first_message,
                    related_information.join("\n")
                )]
            }
        } else {
            vec![]
        }
    }

    fn fix_generation_prompt(&self, selection: &SelectionContext) -> Vec<String> {
        let mut prompts = vec![];
        if selection.above.has_context() {
            let mut above_prompts = vec![];
            above_prompts.push("I have the following code above the selection:".to_owned());
            above_prompts.extend(selection.above.generate_prompt(true));
            prompts.push(above_prompts.join("\n"));
        }
        if selection.below.has_context() {
            let mut below_prompts = vec![];
            below_prompts.push("I have the following code below the selection:".to_owned());
            below_prompts.extend(selection.below.generate_prompt(true));
            prompts.push(below_prompts.join("\n"));
        }
        if selection.range.has_context() {
            let mut range_prompts = vec![];
            range_prompts.push("I have the following code in the selection:".to_owned());
            range_prompts.extend(selection.range.generate_prompt(true));
            prompts.push(range_prompts.join("\n"));
        } else {
            prompts.push("There is no code in the selection.".to_owned());
        }
        let in_range_start_marker = selection.range.start_marker();
        let in_range_end_marker = selection.range.end_marker();
        prompts.push(
            format!(
                "Only change the code inside of the selection, delimited by markers: {in_range_start_marker} and {in_range_end_marker}"
            )
            .to_owned(),
        );
        prompts
    }

    fn edit_generation_prompt(
        &self,
        language: &str,
        selection_with_outline: SelectionWithOutlines,
    ) -> Vec<String> {
        let mut prompts = vec![];
        let has_surrounding_context = selection_with_outline.selection_context.above.has_context()
            || selection_with_outline.selection_context.below.has_context()
            || !selection_with_outline.outline_above.is_empty()
            || !selection_with_outline.outline_below.is_empty();

        let prompt_with_outline = |heading: &str, outline: String, fs_file_path: &str| -> String {
            return vec![
                heading.to_owned(),
                format!("```{language}"),
                format!("// FILEPATH: {fs_file_path}"),
                outline,
                "```".to_owned(),
            ]
            .join("\n");
        };

        let prompt_with_content = |heading: &str, context: &ContextParserInLineEdit| -> String {
            let prompt_parts = context.generate_prompt(has_surrounding_context);
            let mut answer = vec![heading.to_owned()];
            answer.extend(prompt_parts.into_iter());
            answer.join("\n")
        };

        if !selection_with_outline.outline_above.is_empty() {
            prompts.push(prompt_with_outline(
                "I have the following code above:",
                selection_with_outline.outline_above.to_owned(),
                self.editor_request.fs_file_path(),
            ));
        }

        if selection_with_outline.selection_context.above.has_context() {
            prompts.push(prompt_with_content(
                "I have the following code above the selection:",
                &selection_with_outline.selection_context.above,
            ));
        }

        if selection_with_outline.selection_context.below.has_context() {
            prompts.push(prompt_with_content(
                "I have the following code below the selection:",
                &selection_with_outline.selection_context.below,
            ));
        }

        if !selection_with_outline.outline_below.is_empty() {
            prompts.push(prompt_with_outline(
                "I have the following code below:",
                selection_with_outline.outline_below.to_owned(),
                self.editor_request.fs_file_path(),
            ));
        }

        let mut selection_prompt = vec![];
        if selection_with_outline.selection_context.range.has_context() {
            selection_prompt.push("I have the following code in the selection".to_owned());
            selection_prompt.extend(
                selection_with_outline
                    .selection_context
                    .range
                    .generate_prompt(has_surrounding_context)
                    .into_iter(),
            );
        } else {
            let fs_file_path = self.editor_request.fs_file_path();
            selection_prompt.push("I am working with the following code:".to_owned());
            selection_prompt.push(format!("```{language}"));
            selection_prompt.push(format!("// FILEPATH: {fs_file_path}"));
            selection_prompt.push("```".to_owned());
        }
        prompts.push(selection_prompt.join("\n"));
        prompts
    }
}

fn wrap_in_code_block(t: &str, e: &str) -> String {
    let re = regex::Regex::new(r"^\s*(```+)").unwrap();
    let captures = re.captures_iter(e);

    let max_length = captures.map(|cap| cap[1].len() + 1).max().unwrap_or(3);

    let i = "`".repeat(max_length);

    format!("{}{}\n{}\n{}", i, t, e.trim(), i)
}
