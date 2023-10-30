use futures::stream;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, UnboundedSender};

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

use super::prompts;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InLineAgentAnswer {
    pub answer_up_until_now: String,
    pub delta: Option<String>,
    pub state: MessageState,
    // We also send the document symbol in question along the wire
    pub document_symbol: Option<DocumentSymbol>,
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
                            document_symbol.clone(),
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
}

#[cfg(test)]
mod tests {
    use gix::config::file::includes::conditional::Context;

    use crate::{
        agent::llm_funcs::{self, llm::OpenAIModel},
        chunking::{
            languages::TSLanguageParsing,
            text_document::{Position, Range},
            types::{FunctionInformation, FunctionNodeType},
        },
        repo::types::RepoRef,
        webserver::in_line_agent::{SnippetInformation, TextDocumentWeb},
    };

    use super::ProcessInEditorRequest;
    #[test]
    fn test_context_for_in_line_edit() {
        let source_code = r#"
        "#;
        let query = "".to_owned();
        let language = "rust".to_owned();
        let line_count = 0;
        let repo_ref = RepoRef::local("/Users/skcd").expect("test should work");
        let snippet_information = SnippetInformation {
            start_position: Position::new(0, 0, 0),
            end_position: Position::new(0, 0, 0),
        };
        let text_document_web = TextDocumentWeb {
            text: source_code.to_owned(),
            language: language.to_owned(),
            fs_file_path: "/Users/skcd".to_owned(),
            relative_path: "/Users/skcd".to_owned(),
            line_count,
        };
        let thread_id = uuid::Uuid::new_v4();
        let selection = ProcessInEditorRequest {
            query,
            language: language.to_owned(),
            repo_ref,
            snippet_information: snippet_information.clone(),
            text_document_web,
            thread_id,
        };
        // This is the current request selection range
        let selection_range = Range::new(
            snippet_information.start_position,
            snippet_information.end_position,
        );
        // Now we want to get the chunks properly
        // First we get the function blocks along with the ranges we know about
        let ts_language_parsing = TSLanguageParsing::init();
        // we get the function nodes here
        let function_nodes = ts_language_parsing.function_information_nodes(source_code, &language);
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

        let expanded_selection =
            get_expanded_selection_range(function_bodies.as_slice(), selection_range.clone());

        let edit_expansion = EditExpandedSelectionRange {
            expanded_selection: guard_large_expansion(&selection_range, &expanded_selection),
            range_expanded_to_functions: Range::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            function_bodies: fold_function_blocks(
                function_bodies
                    .to_vec()
                    .into_iter()
                    .map(|x| x.clone())
                    .collect(),
            ),
        };

        // these are the missing variables I have to fill in,
        // lines count and the source lines
        let source_lines: Vec<String> = source_code
            .split("/\r\n|\r|\n/g")
            .into_iter()
            .map(|s| s.to_owned())
            .collect();
        generate_context_for_range(
            source_code,
            line_count,
            &selection_range,
            &expanded_selection,
            &edit_expansion.range_expanded_to_functions,
            &language,
            8000,
            source_lines,
            function_bodies.into_iter().map(|fnb| fnb.clone()).collect(),
        );
    }

    // We want to send the above, in-range and the below sections
    pub struct SelectionContext {
        above: ContextParserInLineEdit,
        range: ContextParserInLineEdit,
        below: ContextParserInLineEdit,
    }

    pub struct SelectionLimits {
        above_line_index: i64,
        below_line_index: i64,
        minimum_line_index: i64,
        maximum_line_index: i64,
    }

    pub struct SelectionWithOutlines {
        selection_context: SelectionContext,
        outline_above: String,
        outline_below: String,
    }

    fn generate_context_for_range(
        source_code: &str,
        lines_count: usize,
        original_selection: &Range,
        maintain_range: &Range,
        expanded_range: &Range,
        language: &str,
        character_limit: usize,
        source_lines: Vec<String>,
        function_bodies: Vec<FunctionInformation>,
    ) -> SelectionWithOutlines {
        // Here we will try 2 things:
        // - try to send the whole document as the context first
        // - if that fails, then we try to send the partial document as the
        // context

        let line_count_i64 = <i64>::try_from(lines_count).expect("usize to i64 should not fail");

        // first try with the whole context
        let mut token_tracker = ContextWindowTracker::new(character_limit);
        let selection_context = generate_selection_context(
            source_code,
            line_count_i64,
            original_selection,
            maintain_range,
            &Range::new(Position::new(0, 0, 0), Position::new(lines_count, 0, 0)),
            character_limit,
            language,
            source_lines.to_vec(),
            &mut token_tracker,
        );
        if !(selection_context.above.has_context() && !selection_context.above.is_complete()) {
            return SelectionWithOutlines {
                selection_context,
                outline_above: "".to_owned(),
                outline_below: "".to_owned(),
            };
        }

        // now we try to send just the amount of data we have in the selection
        let mut token_tracker = ContextWindowTracker::new(character_limit);
        let restricted_selection_context = generate_selection_context(
            source_code,
            line_count_i64,
            original_selection,
            maintain_range,
            expanded_range,
            character_limit,
            language,
            source_lines,
            &mut token_tracker,
        );
        let mut outline_above = "".to_owned();
        let mut outline_below = "".to_owned();
        if restricted_selection_context.above.is_complete()
            && restricted_selection_context.below.is_complete()
        {
            let generated_outline = generate_outline_for_range(
                function_bodies,
                expanded_range.clone(),
                language,
                source_code,
            );
            // this is where we make sure we are fitting the above and below
            // into the context window
            let processed_outline = process_outlines(generated_outline, &mut token_tracker);
            outline_above = processed_outline.above;
            outline_below = processed_outline.below;
        }

        SelectionWithOutlines {
            selection_context: restricted_selection_context,
            outline_above,
            outline_below,
        }
    }

    fn process_outlines(
        generated_outline: OutlineForRange,
        context_manager: &mut ContextWindowTracker,
    ) -> OutlineForRange {
        // here we will process the outline again and try to generate it after making
        // sure that it fits in the limit
        let lines_above: Vec<String> = generated_outline
            .above
            .split("/\r\n|\r|\n/g")
            .into_iter()
            .map(|s| s.to_owned())
            .collect();
        let lines_below: Vec<String> = generated_outline
            .below
            .split("/\r\n|\r|\n/g")
            .into_iter()
            .map(|s| s.to_owned())
            .collect();

        let mut processed_above = vec![];
        let mut processed_below = vec![];

        let mut try_add_above_line =
            |line: &str, context_manager: &mut ContextWindowTracker| -> bool {
                if context_manager.line_would_fit(line) {
                    context_manager.add_line(line);
                    processed_above.insert(0, line.to_owned());
                    return true;
                }
                false
            };

        let mut try_add_below_line =
            |line: &str, context_manager: &mut ContextWindowTracker| -> bool {
                if context_manager.line_would_fit(line) {
                    context_manager.add_line(line);
                    processed_below.push(line.to_owned());
                    return true;
                }
                false
            };

        let mut above_index = lines_above.len() - 1;
        let mut below_index = 0;
        let mut can_add_above = true;
        let mut can_add_below = true;

        for index in 0..100 {
            if !can_add_above || (can_add_below && index % 4 == 3) {
                if below_index < lines_below.len()
                    && try_add_below_line(&lines_below[below_index], context_manager)
                {
                    below_index += 1;
                } else {
                    can_add_below = false;
                }
            } else {
                if above_index >= 0
                    && try_add_above_line(&lines_above[above_index], context_manager)
                {
                    above_index -= 1;
                } else {
                    can_add_above = false;
                }
            }
        }

        OutlineForRange {
            above: processed_above.join("\n"),
            below: processed_below.join("\n"),
        }
    }

    struct OutlineForRange {
        above: String,
        below: String,
    }

    fn generate_outline_for_range(
        function_bodies: Vec<FunctionInformation>,
        range_expanded_to_function: Range,
        language: &str,
        source_code: &str,
    ) -> OutlineForRange {
        // Now we try to see if we can expand properly
        let mut terminator = "".to_owned();
        if language == "typescript" {
            terminator = ";".to_owned();
        }

        // we only keep the function bodies which are not too far away from
        // the range we are interested in selecting
        let filtered_function_bodies: Vec<_> = function_bodies
            .to_vec()
            .into_iter()
            .filter_map(|function_body| {
                let fn_body_end_line = function_body.range().end_position().line();
                let fn_body_start_line = function_body.range().start_position().line();
                let range_start_line = range_expanded_to_function.start_position().line();
                let range_end_line = range_expanded_to_function.end_position().line();
                if fn_body_end_line < range_start_line {
                    if range_start_line - fn_body_start_line > 50 {
                        Some(function_body)
                    } else {
                        None
                    }
                } else if fn_body_start_line > range_end_line {
                    if fn_body_end_line - range_end_line > 50 {
                        Some(function_body)
                    } else {
                        None
                    }
                } else {
                    Some(function_body)
                }
            })
            .collect();

        fn build_outline(
            source_code: &str,
            function_bodies: Vec<FunctionInformation>,
            range: Range,
            terminator: &str,
        ) -> OutlineForRange {
            let mut current_index = 0;
            let mut outline_above = "".to_owned();
            let mut end_of_range = range.end_byte();
            let mut outline_below = "".to_owned();

            for function_body in function_bodies.iter() {
                if function_body.range().end_byte() < range.start_byte() {
                    outline_above += source_code
                        .get(current_index..function_body.range().start_byte())
                        .expect("to not fail");
                    outline_above += terminator;
                    current_index = function_body.range().end_byte();
                } else if function_body.range().start_byte() > range.end_byte() {
                    outline_below += source_code
                        .get(end_of_range..function_body.range().start_byte())
                        .expect("to not fail");
                    outline_below += terminator;
                    end_of_range = function_body.range().end_byte();
                }
            }
            outline_above += source_code
                .get(current_index..range.start_byte())
                .expect("to not fail");
            outline_below += source_code
                .get(end_of_range..source_code.len())
                .expect("to not fail");
            OutlineForRange {
                above: outline_above,
                below: outline_below,
            }
        }
        build_outline(
            source_code,
            filtered_function_bodies,
            range_expanded_to_function,
            &terminator,
        )
    }

    fn generate_selection_context(
        source_code: &str,
        line_count: i64,
        original_selection: &Range,
        range_to_maintain: &Range,
        expanded_range: &Range,
        character_limit: usize,
        language: &str,
        lines: Vec<String>,
        mut token_count: &mut ContextWindowTracker,
    ) -> SelectionContext {
        // Change this later on, this is the limits on the characters right
        // now and not the tokens
        let mut in_range = ContextParserInLineEdit::new(
            language.to_owned(),
            "ed8c6549bwf9".to_owned(),
            line_count,
            lines.to_vec(),
        );
        let mut above = ContextParserInLineEdit::new(
            language.to_owned(),
            "abpxx6d04wxr".to_owned(),
            line_count,
            lines.to_vec(),
        );
        let mut below = ContextParserInLineEdit::new(
            language.to_owned(),
            "be15d9bcejpp".to_owned(),
            line_count,
            lines.to_vec(),
        );
        let start_line = range_to_maintain.start_position().line();
        let end_line = range_to_maintain.end_position().line();

        for index in (start_line..=end_line).rev() {
            if !in_range.prepend_line(index, &mut token_count) {
                above.trim(None);
                in_range.trim(Some(original_selection));
                below.trim(None);
                return {
                    SelectionContext {
                        above,
                        range: in_range,
                        below,
                    }
                };
            }
        }

        // Now we can try and expand the above and below ranges, since
        // we have some space for the context
        expand_above_and_below_selections(
            &mut above,
            &mut below,
            &mut token_count,
            SelectionLimits {
                above_line_index: i64::try_from(range_to_maintain.start_position().line())
                    .expect("usize to i64 to work")
                    - 1,
                below_line_index: i64::try_from(range_to_maintain.end_position().line())
                    .expect("usize to i64 to work")
                    + 1,
                minimum_line_index: expanded_range
                    .start_position()
                    .line()
                    .try_into()
                    .expect("usize to i64 to work"),
                maximum_line_index: expanded_range
                    .end_position()
                    .line()
                    .try_into()
                    .expect("usize to i64 to work"),
            },
        );

        // Now we trim out the ranges again and send the result back
        above.trim(None);
        below.trim(None);
        in_range.trim(Some(original_selection));
        SelectionContext {
            above,
            range: in_range,
            below,
        }
    }

    // We are going to expand the above and below ranges to gather more
    // context if possible
    fn expand_above_and_below_selections(
        above: &mut ContextParserInLineEdit,
        below: &mut ContextParserInLineEdit,
        token_count: &mut ContextWindowTracker,
        selection_limits: SelectionLimits,
    ) {
        let mut prepend_line_index = selection_limits.above_line_index;
        let mut append_line_index = selection_limits.below_line_index;
        let mut can_prepend = true;
        let mut can_append = true;
        for iteration in 0..100 {
            if !can_prepend || (can_append && iteration % 4 == 3) {
                // If we're within the allowed range and the append is successful, increase the index
                if append_line_index <= selection_limits.maximum_line_index
                    && below.append_line(
                        append_line_index
                            .try_into()
                            .expect("usize to i64 will not fail"),
                        token_count,
                    )
                {
                    append_line_index += 1;
                } else {
                    // Otherwise, set the flag to stop appending
                    can_append = false;
                }
            } else {
                // If we're within the allowed range and the prepend is successful, decrease the index
                if prepend_line_index >= selection_limits.minimum_line_index
                    && above.prepend_line(
                        prepend_line_index
                            .try_into()
                            .expect("usize to i64 will not fail"),
                        token_count,
                    )
                {
                    prepend_line_index -= 1;
                } else {
                    // Otherwise, set the flag to stop prepending
                    can_prepend = false;
                }
            }
        }
        if prepend_line_index < selection_limits.minimum_line_index {
            above.mark_complete();
        }
        if append_line_index > selection_limits.maximum_line_index {
            below.mark_complete();
        }
    }

    // It can happen that we expand to too large a range, in which case we want
    // to guard against how big it goes
    // our threshold is atmost 30 lines+= expansion
    fn guard_large_expansion(selection_range: &Range, expanded_range: &Range) -> Range {
        let start_line_difference =
            if selection_range.start_position().line() > expanded_range.start_position().line() {
                selection_range.start_position().line() - expanded_range.start_position().line()
            } else {
                expanded_range.start_position().line() - selection_range.start_position().line()
            };
        let end_line_difference =
            if selection_range.end_position().line() > expanded_range.end_position().line() {
                selection_range.end_position().line() - expanded_range.end_position().line()
            } else {
                expanded_range.end_position().line() - selection_range.end_position().line()
            };
        if (start_line_difference + end_line_difference) > 30 {
            // we are going to return the selection range here
            return selection_range.clone();
        } else {
            return expanded_range.clone();
        }
    }

    fn fold_function_blocks(
        mut function_blocks: Vec<FunctionInformation>,
    ) -> Vec<FunctionInformation> {
        // First we sort the function blocks(which are bodies) based on the start
        // index or the end index
        function_blocks.sort_by(|a, b| {
            a.range()
                .start_byte()
                .cmp(&b.range().start_byte())
                .then(b.range().end_byte().cmp(&a.range().end_byte()))
        });

        // Now that these are sorted we only keep the ones which are not overlapping
        // or fully contained in the other one
        let mut filtered_function_blocks = Vec::new();
        let mut index = 0;

        while index < function_blocks.len() {
            function_blocks.push(function_blocks[index].clone());
            let mut iterate_index = index + 1;
            while iterate_index < function_blocks.len()
                && function_blocks[index]
                    .range()
                    .is_contained(&function_blocks[iterate_index].range())
            {
                iterate_index += 1;
            }
            index = iterate_index;
        }

        filtered_function_blocks
    }

    struct EditExpandedSelectionRange {
        expanded_selection: Range,
        range_expanded_to_functions: Range,
        function_bodies: Vec<FunctionInformation>,
    }

    // we are going to get a new range here for our selections
    fn get_expanded_selection_range(
        function_bodies: &[&FunctionInformation],
        selection_range: Range,
    ) -> Range {
        let mut start_position = selection_range.start_position();
        let mut end_position = selection_range.end_position();
        let selection_start_fn_body =
            get_function_bodies_position(function_bodies, selection_range.start_byte());
        let selection_end_fn_body =
            get_function_bodies_position(function_bodies, selection_range.end_byte());

        // What we are trying to do here is expand our selection to cover the whole
        // function if we have to
        if let Some(selection_start_function) = selection_start_fn_body {
            // check if we can expand the range a bit here
            if start_position.to_byte_offset() > selection_start_function.range().start_byte() {
                start_position = selection_start_function.range().start_position();
            }
            // check if the function block ends after our current selection
            if selection_start_function.range().end_byte() > end_position.to_byte_offset() {
                end_position = selection_start_function.range().end_position();
            }
        }
        if let Some(selection_end_function) = selection_end_fn_body {
            // check if we can expand the start position byte here a bit
            if selection_end_function.range().start_byte() < start_position.to_byte_offset() {
                start_position = selection_end_function.range().start_position();
            }
            if selection_end_function.range().end_byte() > end_position.to_byte_offset() {
                end_position = selection_end_function.range().end_position();
            }
        }
        Range::new(start_position, end_position)
    }

    fn get_function_bodies_position<'a>(
        function_blocks: &'a [&'a FunctionInformation],
        byte_offset: usize,
    ) -> Option<&'a FunctionInformation> {
        let possible_function_block = None;
        for function_block in function_blocks {
            // if the end byte for this block is greater than the current byte
            // position and the start byte is greater than the current bytes
            // position as well, we have our function block
            if function_block.range().end_byte() >= byte_offset {
                if function_block.range().start_byte() > byte_offset {
                    return possible_function_block;
                }
            }
        }
        None
    }

    // This will help us keep track of how many tokens we have remaining
    pub struct ContextWindowTracker {
        token_limit: usize,
        total_tokens: usize,
    }

    impl ContextWindowTracker {
        pub fn new(token_limit: usize) -> Self {
            Self {
                token_limit,
                total_tokens: 0,
            }
        }

        pub fn add_tokens(&mut self, tokens: usize) {
            self.total_tokens += tokens;
        }

        pub fn tokens_remaining(&self) -> usize {
            self.token_limit - self.total_tokens
        }

        pub fn line_would_fit(&self, line: &str) -> bool {
            self.total_tokens + line.len() + 1 < self.token_limit
        }

        pub fn add_line(&mut self, line: &str) {
            self.total_tokens += line.len() + 1;
        }
    }

    pub struct ContextParserInLineEdit {
        language: String,
        unique_identifier: String,
        first_line_index: i64,
        last_line_index: i64,
        is_complete: bool,
        non_trim_whitespace_character_count: i64,
        start_marker: String,
        end_marker: String,
        // This is the lines coming from the source
        source_lines: Vec<String>,
        /// This is the lines we are going to use for the context
        lines: Vec<String>,
    }

    impl ContextParserInLineEdit {
        pub fn new(
            language: String,
            unique_identifier: String,
            lines_count: i64,
            source_lines: Vec<String>,
        ) -> Self {
            let comment_style = "//".to_owned();
            Self {
                language,
                unique_identifier: unique_identifier.to_owned(),
                first_line_index: lines_count,
                last_line_index: -1,
                is_complete: false,
                non_trim_whitespace_character_count: 0,
                // we also need to provide the comment style here, lets assume
                // that we are using //
                start_marker: format!("{} BEGIN: {}", &comment_style, unique_identifier),
                end_marker: format!("{} END: {}", &comment_style, unique_identifier),
                source_lines,
                lines: vec![],
            }
        }

        pub fn is_complete(&self) -> bool {
            self.is_complete
        }

        pub fn mark_complete(&mut self) {
            self.is_complete = true;
        }

        pub fn has_context(&self) -> bool {
            if self.lines.len() == 0 || self.non_trim_whitespace_character_count == 0 {
                false
            } else {
                !self.lines.is_empty()
            }
        }

        pub fn prepend_line(
            &mut self,
            line_index: usize,
            character_limit: &mut ContextWindowTracker,
        ) -> bool {
            let line_text = self.source_lines[line_index].to_owned();
            if !character_limit.line_would_fit(&line_text) {
                return false;
            }

            self.first_line_index = std::cmp::min(self.first_line_index, line_index as i64);
            self.last_line_index = std::cmp::max(self.last_line_index, line_index as i64);

            character_limit.add_line(&line_text);
            self.non_trim_whitespace_character_count += line_text.trim().len() as i64;
            self.lines.insert(0, line_text);

            true
        }

        pub fn append_line(
            &mut self,
            line_index: usize,
            character_limit: &mut ContextWindowTracker,
        ) -> bool {
            let line_text = self.source_lines[line_index].to_owned();
            if !character_limit.line_would_fit(&line_text) {
                return false;
            }

            self.first_line_index = std::cmp::min(self.first_line_index, line_index as i64);
            self.last_line_index = std::cmp::max(self.last_line_index, line_index as i64);

            character_limit.add_line(&line_text);
            self.non_trim_whitespace_character_count += line_text.trim().len() as i64;
            self.lines.push(line_text);

            true
        }

        pub fn trim(&mut self, range: Option<&Range>) {
            // now we can begin trimming it on a range if appropriate and then
            // do things properly
            let last_line_index = if let Some(range) = range.clone() {
                if self.last_line_index
                    < range
                        .start_position()
                        .line()
                        .try_into()
                        .expect("usize to i64 not fail")
                {
                    self.last_line_index
                } else {
                    range
                        .start_position()
                        .line()
                        .try_into()
                        .expect("usize to i64 not fail")
                }
            } else {
                self.last_line_index
            };
            for _ in self.first_line_index..last_line_index {
                if self.lines.len() > 0 && self.lines[0].trim().len() == 0 {
                    self.first_line_index += 1;
                    self.lines.remove(0);
                }
            }

            let first_line_index = if let Some(range) = range {
                if self.first_line_index
                    > range
                        .end_position()
                        .line()
                        .try_into()
                        .expect("usize to i64 not fail")
                {
                    self.first_line_index
                } else {
                    range
                        .end_position()
                        .line()
                        .try_into()
                        .expect("usize to i64 not fail")
                }
            } else {
                self.first_line_index
            };

            for _ in first_line_index..self.last_line_index {
                if self.lines.len() > 0 && self.lines[self.lines.len() - 1].trim().len() == 0 {
                    self.last_line_index -= 1;
                    self.lines.pop();
                }
            }
        }
    }
}
