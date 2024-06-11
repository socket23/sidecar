//! Contains the symbol type and how its structred and what kind of operations we
//! can do with it, its a lot of things but the end goal is that each symbol in the codebase
//! can be represented by some entity, and that's what we are storing over here
//! Inside each symbol we also have the various implementations of it, which we always
//! keep track of and whenever a question is asked we forward it to all the implementations
//! and select the ones which are necessary.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use derivative::Derivative;
use futures::{stream, StreamExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::info;

use crate::{
    agentic::{
        symbol::{
            events::edit::SymbolToEditRequest,
            identifier::Snippet,
            ui_event::{SymbolEventProbeRequest, SymbolEventSubStep, SymbolEventSubStepRequest},
        },
        tool::{
            code_symbol::{
                important::{
                    CodeSubSymbolProbingResult, CodeSymbolFollowAlongForProbing,
                    CodeSymbolProbingSummarize, CodeSymbolWithThinking,
                },
                models::anthropic::ProbeNextSymbol,
            },
            lsp::open_file::OpenFileResponse,
        },
    },
    chunking::{text_document::Range, types::OutlineNodeContent},
};

use super::{
    errors::SymbolError,
    events::{
        edit::SymbolToEdit,
        probe::{SymbolToProbeHistory, SymbolToProbeRequest},
        types::{AskQuestionRequest, SymbolEvent},
    },
    helpers::split_file_content_into_parts,
    identifier::{LLMProperties, MechaCodeSymbolThinking, SymbolIdentifier},
    tool_box::ToolBox,
    tool_properties::ToolProperties,
    ui_event::UIEventWithID,
};

const BUFFER_LIMIT: usize = 100;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolSubStepUpdate {
    sybmol: SymbolIdentifier,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolLocation {
    snippet: Snippet,
    symbol_identifier: SymbolIdentifier,
}

impl SymbolLocation {
    pub fn new(symbol_identifier: SymbolIdentifier, snippet: Snippet) -> Self {
        Self {
            snippet,
            symbol_identifier,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolEventRequest {
    symbol: SymbolIdentifier,
    event: SymbolEvent,
    tool_properties: ToolProperties,
}

impl SymbolEventRequest {
    pub fn event(&self) -> &SymbolEvent {
        &self.event
    }

    pub fn symbol(&self) -> &SymbolIdentifier {
        &self.symbol
    }

    pub fn remove_event(self) -> SymbolEvent {
        self.event
    }

    pub fn get_tool_properties(&self) -> &ToolProperties {
        &self.tool_properties
    }
}

impl SymbolEventRequest {
    pub fn new(
        symbol: SymbolIdentifier,
        event: SymbolEvent,
        tool_properties: ToolProperties,
    ) -> Self {
        Self {
            symbol,
            event,
            tool_properties,
        }
    }

    pub fn outline(symbol: SymbolIdentifier, tool_properties: ToolProperties) -> Self {
        Self {
            symbol,
            event: SymbolEvent::Outline,
            tool_properties,
        }
    }

    pub fn ask_question(
        symbol: SymbolIdentifier,
        question: String,
        tool_properties: ToolProperties,
    ) -> Self {
        Self {
            symbol,
            event: SymbolEvent::AskQuestion(AskQuestionRequest::new(question)),
            tool_properties,
        }
    }

    pub fn probe_request(
        symbol: SymbolIdentifier,
        request: SymbolToProbeRequest,
        tool_properties: ToolProperties,
    ) -> Self {
        Self {
            symbol,
            event: SymbolEvent::Probe(request),
            tool_properties,
        }
    }
}

#[derive(Debug)]
pub enum SymbolEventResponse {
    TaskDone(String),
}

impl SymbolEventResponse {
    pub fn to_string(self) -> String {
        match self {
            Self::TaskDone(reply) => reply,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditedCodeSymbol {
    original_code: String,
    edited_code: String,
}

impl EditedCodeSymbol {
    pub fn new(original_code: String, edited_code: String) -> Self {
        Self {
            original_code,
            edited_code,
        }
    }
}

/// The symbol is going to spin in the background and keep working on things
/// is this how we want it to work???
/// ideally yes, cause its its own process which will work in the background
/// one of the keys things here is that we want this to be a arcable and clone friendly
/// since we are sending many such events to it at the same time
#[derive(Derivative)]
#[derivative(PartialEq, Eq, Debug, Clone)]
pub struct Symbol {
    symbol_identifier: SymbolIdentifier,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    hub_sender: UnboundedSender<(
        SymbolEventRequest,
        String,
        // we can await on the receiver
        tokio::sync::oneshot::Sender<SymbolEventResponse>,
    )>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    tools: Arc<ToolBox>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    // TODO(skcd): this is a skill issue right here
    // we do not want to clone so aggresively here, we should be able to work
    // with just references somehow if we were not mutating our state so much
    mecha_code_symbol: Arc<MechaCodeSymbolThinking>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    llm_properties: LLMProperties,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    ui_sender: UnboundedSender<UIEventWithID>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    tool_properties: ToolProperties,
}

impl Symbol {
    pub async fn new(
        symbol_identifier: SymbolIdentifier,
        mecha_code_symbol: MechaCodeSymbolThinking,
        // this can be used to talk to other symbols and get them
        // to act on certain things
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            String,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
        tools: Arc<ToolBox>,
        llm_properties: LLMProperties,
        ui_sender: UnboundedSender<UIEventWithID>,
        request_id: String,
        tool_properties: ToolProperties,
    ) -> Result<Self, SymbolError> {
        let symbol = Self {
            mecha_code_symbol: Arc::new(mecha_code_symbol),
            symbol_identifier,
            hub_sender,
            tools,
            llm_properties,
            ui_sender,
            tool_properties,
        };
        // grab the implementations of the symbol
        // TODO(skcd): We also have to grab the diagnostics and auto-start any
        // process which we might want to
        symbol.grab_implementations(&request_id).await?;
        Ok(symbol)
    }

    fn fs_file_path(&self) -> &str {
        self.mecha_code_symbol.fs_file_path()
    }

    fn symbol_name(&self) -> &str {
        self.mecha_code_symbol.symbol_name()
    }

    // find the name of the sub-symbol
    pub async fn find_subsymbol_in_range(
        &self,
        range: &Range,
        fs_file_path: &str,
    ) -> Option<String> {
        self.mecha_code_symbol
            .find_symbol_in_range(range, fs_file_path)
            .await
    }

    async fn get_outline(&self, request_id: String) -> Result<String, SymbolError> {
        // to grab the outline first we need to understand the definition snippet
        // of the node and then create it appropriately
        // First thing we want to do here is to find the symbols which are present
        // in the file and get the one which corresponds to us, once we have that
        // we go to all the implementations and grab them as well and generate
        // the outline which is required
        self.tools
            .outline_nodes_for_symbol(self.fs_file_path(), self.symbol_name(), &request_id)
            .await
    }

    async fn grab_implementations(&self, request_id: &str) -> Result<(), SymbolError> {
        let snippet_file_path: Option<String>;
        {
            snippet_file_path = self
                .mecha_code_symbol
                .get_snippet()
                .await
                .map(|snippet| snippet.file_path().to_owned());
        }
        if let Some(snippet_file_path) = snippet_file_path {
            // We first rerank the snippets and then ask the llm for which snippets
            // need to be edited
            // this is not perfect as there is heirarchy in the symbols which we might have
            // to model at some point (but not sure if we really need to do)
            // assuming: LLMs do not need more granular output per class (if there are functions
            // which need to change, we can catch them in the refine step)
            // we break this apart in pieces so the llm can do better
            // we iterate until the llm has listed out all the functions which
            // need to be changed
            // and we are anyways tracking the changes which are happening
            // in the first level of iteration
            // PS: we can ask for a refinement step after this which forces the
            // llm to generate more output for a step using the context it has
            let implementations = self
                .tools
                .go_to_implementation(
                    &snippet_file_path,
                    self.symbol_identifier.symbol_name(),
                    request_id,
                )
                .await?;
            let unique_files = implementations
                .get_implementation_locations_vec()
                .iter()
                .map(|implementation| implementation.fs_file_path().to_owned())
                .collect::<HashSet<String>>();
            let cloned_tools = self.tools.clone();
            // once we have the unique files we have to request to open these locations
            let file_content_map = stream::iter(unique_files.clone())
                .map(|file_path| (file_path, cloned_tools.clone()))
                .map(|(file_path, tool_box)| async move {
                    let file_path = file_path.to_owned();
                    let file_content = tool_box.file_open(file_path.to_owned(), request_id).await;
                    // we will also force add the file to the symbol broker
                    if let Ok(file_content) = &file_content {
                        let _ = tool_box
                            .force_add_document(
                                &file_path,
                                file_content.contents_ref(),
                                &file_content.language(),
                            )
                            .await;
                    }
                    (file_path, file_content)
                })
                // limit how many files we open in parallel
                .buffer_unordered(4)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<HashMap<String, Result<OpenFileResponse, SymbolError>>>();
            // grab the outline nodes as well
            let outline_nodes = stream::iter(unique_files)
                .map(|file_path| (file_path, cloned_tools.clone()))
                .map(|(file_path, tool_box)| async move {
                    (
                        file_path.to_owned(),
                        // TODO(skcd): One of the bugs here is that we are also
                        // returning the node containing the full outline
                        // which wins over any other node, so this breaks the rest of the
                        // flow, what should we do here??
                        tool_box.get_outline_nodes(&file_path).await,
                    )
                })
                .buffer_unordered(1)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<HashMap<String, Option<Vec<OutlineNodeContent>>>>();
            // Once we have the file content map, we can read the ranges which we are
            // interested in and generate the implementation areas
            // we have to figure out how to handle updates etc as well, but we will get
            // to that later
            // TODO(skcd): This is probably wrong since we need to calculate the bounding box
            // for the function
            let implementation_content = implementations
                .get_implementation_locations_vec()
                .iter()
                .filter_map(|implementation| {
                    let file_path = implementation.fs_file_path().to_owned();
                    let range = implementation.range();
                    // if file content is empty, then we do not add this to our
                    // implementations
                    let file_content = file_content_map.get(&file_path);
                    if let Some(Ok(ref file_content)) = file_content {
                        let outline_nodes_for_range = outline_nodes
                            .get(&file_path)
                            .map(|outline_nodes| {
                                if let Some(outline_nodes) = outline_nodes {
                                    // grab the first outline node which we find which contains the range we are interested in
                                    // this will always give us the biggest range
                                    let first_outline_node = outline_nodes
                                        .iter()
                                        .filter(|outline_node| {
                                            outline_node.range().contains_check_line(range)
                                        })
                                        .next();
                                    first_outline_node.map(|outline_node| outline_node.clone())
                                } else {
                                    None
                                }
                            })
                            .flatten();
                        match (
                            file_content.content_in_range(&range),
                            outline_nodes_for_range,
                        ) {
                            (Some(content), Some(outline_nodes)) => Some(Snippet::new(
                                self.symbol_identifier.symbol_name().to_owned(),
                                range.clone(),
                                file_path,
                                content,
                                outline_nodes,
                            )),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            // we update the snippets we have stored here into the symbol itself
            {
                self.mecha_code_symbol
                    .set_implementations(implementation_content)
                    .await;
            }
        }
        Ok(())
    }

    // We are asked on the complete symbol a question
    // - we have to first find the sub-symbol we are interested in
    // - then ask it the probing question
    // - once we have the probing question, we send over the request and wait for the response
    // - and finally we stop doing this.
    // TODO(skcd): We need to cache the results for each request here to the symbol
    // and if there is an error, then we remove it from the cache and what if something
    // is poll waiting on it, can we pass the progress made up-until now to the calling
    // request, this way we can store the progress
    async fn probe_request(
        &self,
        request: SymbolToProbeRequest,
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            String,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
        request_id: String,
    ) -> Result<String, SymbolError> {
        let request_id_ref = &request_id;
        // we can do a cache check here if we already have the answer or are working
        // on the similar request
        // First we refresh our state over here
        self.refresh_state(request_id_ref.to_owned()).await;

        let history = request.history();
        let history_slice = request.history_slice();
        let original_request = request.original_request();
        let history_ref = &history;
        let query = request.probe_request();
        let symbol_name = self.mecha_code_symbol.symbol_name();
        let tool_properties_ref = &self.tool_properties;

        let snippets = self.mecha_code_symbol.get_implementations().await;
        info!(event_name = "refresh_state", symbol_name = symbol_name,);
        // - sub-symbol selection for probing
        let sub_symbol_request = self
            .tools
            .probe_sub_symbols(
                snippets,
                &request,
                self.llm_properties.llm().clone(),
                self.llm_properties.provider().clone(),
                self.llm_properties.api_key().clone(),
                request_id_ref,
            )
            .await?;
        let _ = self.ui_sender.send(UIEventWithID::sub_symbol_step(
            request_id_ref.to_owned(),
            SymbolEventSubStepRequest::new(
                self.symbol_identifier.clone(),
                SymbolEventSubStep::Probe(SymbolEventProbeRequest::SubSymbolSelection),
            ),
        ));
        println!("Sub symbol request: {:?}", &sub_symbol_request);

        // - ask if we should probe the sub-symbols here
        let filtering_response = stream::iter(
            sub_symbol_request
                .snippets_to_probe_ordered()
                .into_iter()
                .map(|snippet_with_reason| {
                    let reason = snippet_with_reason.reason().to_owned();
                    let snippet = snippet_with_reason.remove_snippet();
                    (reason, snippet, self.llm_properties.clone())
                }),
        )
        .map(|(reason, snippet, _llm_properties)| async move {
            // TODO(skcd): This asking of question does not feel necessary
            // since we are doing pre-filtering before
            // Now depending on the response here we can exlcude/include
            // the symbols which we want to follow and ask for more information
            // let response = self
            //     .tools
            //     .should_follow_subsymbol_for_probing(
            //         &snippet,
            //         &reason,
            //         history_ref,
            //         query,
            //         llm_properties.llm().clone(),
            //         llm_properties.provider().clone(),
            //         llm_properties.api_key().clone(),
            //     )
            //     .await;
            // println!("Response: {:?}", &response);
            // (reason, snippet, response)
            (reason.to_owned(), snippet, reason)
        })
        .buffer_unordered(BUFFER_LIMIT)
        .collect::<Vec<_>>()
        .await;

        println!("Sub symbol fetching: {:?}", &filtering_response);

        info!("sub symbol fetching done");

        // println!("Snippet filtering response: {:?}", &filtering_response);

        // let filtered_snippets = filtering_response
        //     .into_iter()
        //     .filter_map(|(reason, snippet, probe_deeper)| match probe_deeper {
        //         Ok(probe_deeper) => {
        //             if probe_deeper.should_follow() {
        //                 Some((reason, snippet, probe_deeper.thinking().to_owned()))
        //             } else {
        //                 None
        //             }
        //         }
        //         Err(_) => None,
        //     })
        //     .collect::<Vec<_>>();
        let filtered_snippets = filtering_response;
        let snippet_to_symbols_to_follow = stream::iter(filtered_snippets)
            .map(|(_, snippet, reason_to_follow)| async move {
                let response = self
                    .tools
                    .probe_deeper_in_symbol(
                        &snippet,
                        &reason_to_follow,
                        history_ref,
                        query,
                        self.llm_properties.llm().clone(),
                        self.llm_properties.provider().clone(),
                        self.llm_properties.api_key().clone(),
                        request_id_ref,
                    )
                    .await;
                (snippet, response)
            })
            .buffer_unordered(BUFFER_LIMIT)
            .collect::<Vec<_>>()
            .await;
        let _ = self.ui_sender.send(UIEventWithID::sub_symbol_step(
            request_id_ref.to_owned(),
            SymbolEventSubStepRequest::new(
                self.symbol_identifier.clone(),
                SymbolEventSubStep::Probe(SymbolEventProbeRequest::ProbeDeeperSymbol),
            ),
        ));

        println!(
            "Snippet to symbols to follow: {:?}",
            snippet_to_symbols_to_follow
        );

        // Now for each snippet we want to grab the definition of the symbol it belongs
        let snippet_to_follow_with_definitions =
            stream::iter(snippet_to_symbols_to_follow.into_iter().filter_map(
                |(snippet, response)| match response {
                    Ok(response) => Some((snippet, response)),
                    Err(_) => None,
                },
            ))
            .map(|(snippet, response)| async move {
                let referred_snippet = &snippet;
                // we want to parse the reponse here properly and get back the
                // snippets which are the most important by going to their definitions
                let symbols_to_follow_list = response.symbol_list();
                let definitions_to_follow = stream::iter(symbols_to_follow_list)
                    .map(|symbol_to_follow| async move {
                        println!(
                            "Go to definition using symbol: {:?} {} {} {}",
                            referred_snippet.range(),
                            symbol_to_follow.file_path(),
                            symbol_to_follow.line_content(),
                            symbol_to_follow.name()
                        );
                        let definitions_for_snippet = self
                            .tools
                            .go_to_definition_using_symbol(
                                referred_snippet.range(),
                                symbol_to_follow.file_path(),
                                symbol_to_follow.line_content(),
                                symbol_to_follow.name(),
                                request_id_ref,
                            )
                            .await;
                        (symbol_to_follow, definitions_for_snippet)
                    })
                    .buffer_unordered(BUFFER_LIMIT)
                    .collect::<Vec<_>>()
                    .await;
                // Now that we have this, we can ask the LLM to generate the next set of probe-queries
                // if required unless it already has the answer
                (snippet, definitions_to_follow)
            })
            // we go through the snippets one by one
            .buffered(1)
            .collect::<Vec<_>>()
            .await;

        println!(
            "Snippet to follow with definitions: {:?}",
            &snippet_to_follow_with_definitions
        );

        // - ask the followup question to the symbol containing the definition we are interested in
        // - we can concat the various questions about the symbols together and just ask the symbol
        // the question maybe?
        let probe_results = stream::iter(snippet_to_follow_with_definitions)
            .map(|(snippet, ask_question_symbol_hint)| async move {
                let questions_with_definitions = ask_question_symbol_hint
                    .into_iter()
                    .filter_map(|(ask_question_hint, definition_path_and_range)| {
                        match definition_path_and_range {
                            Ok(definition_path_and_range) => {
                                Some((ask_question_hint, definition_path_and_range))
                            }
                            Err(_) => None,
                        }
                    })
                    .collect::<Vec<_>>();

                // What we want to create is this: CodeSymbolFollowAlongForProbing
                let file_content = self
                    .tools
                    .file_open(snippet.file_path().to_owned(), request_id_ref)
                    .await;
                if let Err(_) = file_content {
                    return Err(SymbolError::ExpectedFileToExist);
                }
                let snippet_range = snippet.range();
                let file_content = file_content.expect("if let Err to work").contents();
                let file_contents_ref = &file_content;
                let probe_results = stream::iter(questions_with_definitions)
                    .map(|(_, definitions)| async move {
                        let next_symbol_link = definitions.0;
                        let definitions = definitions.1;
                        // we might also want to grab some kind of outline for the symbol hint we are gonig to be using
                        // we are missing the code above, code below and the in selection
                        // should we recosinder the snippet over here or maybe we just keep it as it is
                        let (code_above, code_below, code_in_selection) =
                            split_file_content_into_parts(file_contents_ref, snippet_range);
                        let definition_names = definitions
                            .iter()
                            .map(|definition| definition.1.to_owned())
                            .collect::<Vec<_>>();
                        let definition_outlines = definitions
                            .into_iter()
                            .map(|definition| definition.2)
                            .collect::<Vec<_>>();
                        let request = CodeSymbolFollowAlongForProbing::new(
                            history_ref.to_owned(),
                            self.mecha_code_symbol.symbol_name().to_owned(),
                            self.mecha_code_symbol.fs_file_path().to_owned(),
                            self.tools
                                .detect_language(self.mecha_code_symbol.fs_file_path())
                                .unwrap_or("".to_owned()),
                            definition_names,
                            definition_outlines,
                            code_above,
                            code_below,
                            code_in_selection,
                            self.llm_properties.llm().clone(),
                            self.llm_properties.provider().clone(),
                            self.llm_properties.api_key().clone(),
                            query.to_owned(),
                            next_symbol_link,
                        );
                        let probe_result = self
                            .tools
                            .next_symbol_should_probe_request(request, request_id_ref)
                            .await;
                        // Now we get the response from here and we can decide what to do with it
                        probe_result
                    })
                    .buffer_unordered(100)
                    .collect::<Vec<_>>()
                    .await;

                Ok((snippet, probe_results))

                // To be very honest here we can ask if we want to send one more probe
                // message over here if this is useful to find the changes
                // TODO(skcd): Pick this up from here for sending over a request to the LLM
                // to figure out if:
                // - A: we have all the information required to answer
                // - B: if we need to go deeper into the symbol for looking into the information
                // If this is not useful we can stop over here
            })
            .buffer_unordered(BUFFER_LIMIT)
            .collect::<Vec<_>>()
            .await;

        // - depending on the probe result we can either
        // - - send one more request at this point
        // - - or we have the answer to the user query
        // - questions: what if one of the probes here tells us that we have the answer already?
        let probe_results = probe_results
            .into_iter()
            .filter_map(|probe_result| match probe_result {
                Ok(probe_result) => Some(probe_result),
                Err(_) => None,
            })
            .collect::<Vec<_>>();

        println!("Probe results: {:?}", &probe_results);

        let hub_sender_ref = &hub_sender;

        let probe_answers = stream::iter(probe_results)
            .map(|probe_result| async move {
                let snippet = probe_result.0;
                let snippet_file_path = snippet.file_path();
                let snippet_content = snippet.content();
                let probe_results = probe_result
                    .1
                    .into_iter()
                    .filter_map(|s| match s {
                        Ok(s) => Some(s),
                        Err(_) => None,
                    })
                    .collect::<Vec<_>>();
                let probing_results = stream::iter(probe_results)
                    .map(|probe_result| async move {
                        match probe_result {
                            ProbeNextSymbol::AnswerUserQuery(answer) => Ok(answer),
                            ProbeNextSymbol::Empty => Ok("".to_owned()),
                            ProbeNextSymbol::ShouldFollow(should_follow_request) => {
                                let file_path = should_follow_request.file_path();
                                let symbol_name = should_follow_request.name();
                                let reason = should_follow_request.reason();
                                let symbol_identifier =
                                    SymbolIdentifier::with_file_path(symbol_name, file_path);
                                let mut history = history_slice.to_vec();
                                let new_history_element = SymbolToProbeHistory::new(
                                    symbol_name.to_owned(),
                                    snippet_file_path.to_owned(),
                                    snippet_content.to_owned(),
                                    query.to_owned(),
                                );
                                history.push(new_history_element);
                                let symbol_to_probe_request = SymbolToProbeRequest::new(
                                    symbol_identifier.clone(),
                                    reason.to_owned(),
                                    original_request.to_owned(),
                                    history,
                                );
                                println!(
                                    "Probing: {:?} with reason: {}",
                                    &symbol_identifier, reason
                                );
                                let (sender, receiver) = tokio::sync::oneshot::channel();
                                let _ = hub_sender_ref.clone().send((
                                    SymbolEventRequest::probe_request(
                                        symbol_identifier,
                                        symbol_to_probe_request,
                                        tool_properties_ref.clone(),
                                    ),
                                    uuid::Uuid::new_v4().to_string(),
                                    sender,
                                ));
                                let response = receiver.await.map(|response| response.to_string());
                                response
                            }
                            ProbeNextSymbol::WrongPath(wrong_path) => Ok(wrong_path),
                        }
                    })
                    .buffer_unordered(BUFFER_LIMIT)
                    .collect::<Vec<_>>()
                    .await;
                (snippet, probing_results)
            })
            .buffer_unordered(BUFFER_LIMIT)
            .collect::<Vec<_>>()
            .await;

        // Lets be dumb over here and just paste the replies we are getting at this point with some hint about the symbol
        // this way we make it a problem for the LLM to answer it at the end
        let sub_symbol_probe_result = probe_answers
            .into_iter()
            .filter_map(|probing_result| {
                let snippet = probing_result.0;
                let answers = probing_result
                    .1
                    .into_iter()
                    .filter_map(|answer| match answer {
                        Ok(answer) => Some(answer),
                        Err(_) => None,
                    })
                    .collect::<Vec<_>>();
                if answers.is_empty() {
                    None
                } else {
                    Some(CodeSubSymbolProbingResult::new(
                        snippet.symbol_name().to_owned(),
                        snippet.file_path().to_owned(),
                        answers,
                        snippet.content().to_owned(),
                    ))
                }
            })
            .collect::<Vec<_>>();

        if sub_symbol_probe_result.is_empty() {
            Ok("no information found to reply to the user query".to_owned())
        } else {
            // summarize the results over here properly
            let request = CodeSymbolProbingSummarize::new(
                query.to_owned(),
                history.to_owned(),
                self.mecha_code_symbol.symbol_name().to_owned(),
                self.get_outline(request_id_ref.to_owned()).await?,
                self.mecha_code_symbol.fs_file_path().to_owned(),
                sub_symbol_probe_result,
                self.llm_properties.llm().clone(),
                self.llm_properties.provider().clone(),
                self.llm_properties.api_key().clone(),
            );
            let result = self
                .tools
                .probing_results_summarize(request, request_id_ref)
                .await;
            let _ = self.ui_sender.send(UIEventWithID::probe_answer_event(
                request_id_ref.to_owned(),
                self.symbol_identifier.clone(),
                result
                    .as_ref()
                    .map(|s| s.to_owned())
                    .unwrap_or("Error with probing answer".to_owned()),
            ));
            println!(
                "Probing finished for {} with result: {:?}",
                &self.mecha_code_symbol.symbol_name(),
                &result
            );
            result
        }
    }

    /// Refreshing the state here implies the following:
    /// - we figure out all the implementations again and also
    /// our core snippet again
    /// - this way even if there have been changes we are almost always
    /// correct
    async fn refresh_state(&self, request_id: String) {
        // do we really have to do this? or can we get away from this just by
        // not worrying about things?
        let snippet = self
            .tools
            .find_snippet_for_symbol(self.fs_file_path(), self.symbol_name(), &request_id)
            .await;
        // if we do have a snippet here which is present update it, otherwise its a pretty
        // bad sign that we had the snippet before but do not have it now
        if let Ok(snippet) = snippet {
            self.mecha_code_symbol.set_snippet(snippet.clone()).await;
            let _ = self.ui_sender.send(UIEventWithID::symbol_location(
                request_id.to_owned(),
                SymbolLocation::new(self.symbol_identifier.clone(), snippet),
            ));
        }
        // now grab the implementations again
        let _ = self.grab_implementations(&request_id).await;
    }

    async fn generate_initial_request(
        &self,
        request_id: String,
        original_request: &str,
    ) -> Result<SymbolEventRequest, SymbolError> {
        // this is a very big block because of the LLM request, but lets see how
        // this plays out in practice
        self.mecha_code_symbol
            .initial_request(
                self.tools.clone(),
                original_request,
                self.llm_properties.clone(),
                request_id,
                &self.tool_properties,
            )
            .await
    }

    // The protocol here is that the questions are just plain text, its on the symbol
    // to decide if it needs to collect more information or make changes, we need to carefully
    // figure that out over here
    // what tools do we provide to the symbol for this?
    async fn _answer_question(&self, _question: &str) -> Result<SymbolEventRequest, SymbolError> {
        // The idea here we want to do is:
        // - We first ask which symbols we want to go towards and also do a global search
        // - We then do a question for any followup changes which we need to do on these other symbols (ask them a query and wait for the result)
        // - Use the returned data to create the final edit or answer question here as required
        // - Finally if our shape has changed we need to schedule followups
        todo!("we need to make sure this works")
    }

    // TODO(skcd): Handle the cases where the outline is within a symbol and spread
    // across different lines (as is the case in typescript and python)
    // for now we are focussing on rust
    pub async fn grab_context_for_editing(
        &self,
        subsymbol: &SymbolToEdit,
        request_id: &str,
    ) -> Result<Vec<String>, SymbolError> {
        let file_content = self
            .tools
            .get_file_content(&subsymbol.fs_file_path())
            .await?;
        let symbol_to_edit = self.tools.find_sub_symbol_to_edit(subsymbol).await?;
        println!(
            "symbol::grab_context_for_editing::symbol_to_edit\n{:?}",
            &symbol_to_edit
        );
        let selection_range = symbol_to_edit.range();
        let _language = self
            .tools
            .detect_language(&subsymbol.fs_file_path())
            .unwrap_or("".to_owned());
        // we have 2 tools which can be used here and they are both kind of interesting:
        // - one of them is the grab definitions which are relevant
        // - one of them is the global context search
        // - first we try to check if the sub-symbol exists in the file
        let interested_defintiions = self
            .tools
            .gather_important_symbols_with_definition(
                symbol_to_edit.fs_file_path(),
                &file_content,
                selection_range,
                self.llm_properties.llm().clone(),
                self.llm_properties.provider().clone(),
                self.llm_properties.api_key().clone(),
                &subsymbol.instructions().join("\n"),
                self.hub_sender.clone(),
                request_id,
                &self.tool_properties,
            )
            .await?;
        let codebase_wide_search: Vec<Option<(CodeSymbolWithThinking, String)>> = vec![];
        // disabling this for now
        // let codebase_wide_search = self
        //     .tools
        //     .utlity_symbols_search(
        //         &subsymbol.instructions().join("\n"),
        //         interested_defintiions
        //             .iter()
        //             .filter_map(|interested_symbol| {
        //                 if let Some((code_symbol, _)) = interested_symbol {
        //                     Some(code_symbol)
        //                 } else {
        //                     None
        //                 }
        //             })
        //             .collect::<Vec<_>>()
        //             .as_slice(),
        //         &symbol_to_edit,
        //         &file_content,
        //         &subsymbol.fs_file_path(),
        //         self.mecha_code_symbol.user_context(),
        //         &language,
        //         self.llm_properties.llm().clone(),
        //         self.llm_properties.provider().clone(),
        //         self.llm_properties.api_key().clone(),
        //         self.hub_sender.clone(),
        //         request_id,
        //     )
        //     .await?;

        // cool now we have all the symbols which are necessary for making the edit
        // and more importantly we have all the context which is required
        // we can send the edit request
        // this is the planning stage at this point, now we can begin the editing
        let outlines = interested_defintiions
            .iter()
            .filter_map(|interesed_definitions| {
                if let Some(interesed_definitions) = interesed_definitions {
                    Some(interesed_definitions.1.to_owned())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .chain(
                codebase_wide_search
                    .iter()
                    .filter_map(|codebase_wide_definitions| {
                        if let Some(interested_definitions) = codebase_wide_definitions {
                            Some(interested_definitions.1.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .collect::<Vec<_>>();
        Ok(outlines)
    }

    async fn edit_code(
        &self,
        sub_symbol: &SymbolToEdit,
        context: Vec<String>,
        request_id: &str,
    ) -> Result<EditedCodeSymbol, SymbolError> {
        let file_content = self
            .tools
            .get_file_content(&sub_symbol.fs_file_path())
            .await?;
        let symbol_to_edit = self.tools.find_sub_symbol_to_edit(sub_symbol).await?;
        let content = symbol_to_edit.content().to_owned();
        let response = self
            .tools
            .code_edit(
                sub_symbol.fs_file_path(),
                &file_content,
                symbol_to_edit.range(),
                context.join("\n"),
                sub_symbol.instructions().join("\n"),
                self.llm_properties.llm().clone(),
                self.llm_properties.provider().clone(),
                self.llm_properties.api_key().clone(),
                request_id,
            )
            .await?;
        Ok(EditedCodeSymbol::new(content, response))
    }

    // we are going to edit the symbols over here
    // some challenges:
    // - we want this to be fully parallel first of all
    // - we also have to do the follow-up fixes on this when we are done editing
    // - we have to look at the lsp information as well
    // - we also want it to be fully parallel
    async fn edit_implementations(
        &self,
        edit_request: SymbolToEditRequest,
        request_id: String,
    ) -> Result<(), SymbolError> {
        // here we might want to edit ourselves or generate new code depending
        // on the scope of the changes being made
        let sub_symbols_to_edit = edit_request.symbols();
        let request_id_ref = &request_id;
        println!(
            "symbol::edit_implementations::sub_symbols::({}).len({})",
            self.symbol_name(),
            sub_symbols_to_edit.len()
        );
        // edit requires the following:
        // - gathering context for the symbols which the definitions or outlines are required
        // - making the edits
        // - following the changed symbol to check on the references and wherever its being used
        for sub_symbol_to_edit in sub_symbols_to_edit.into_iter() {
            println!(
                "symbol::edit_implementation::sub_symbol_to_edit::({}):\n{:?}",
                sub_symbol_to_edit.symbol_name(),
                &sub_symbol_to_edit,
            );
            let context_for_editing = dbg!(
                self.grab_context_for_editing(&sub_symbol_to_edit, request_id_ref)
                    .await
            )?;
            // always return the original code which was present here in case of rollbacks
            let edited_code = self
                .edit_code(
                    &sub_symbol_to_edit,
                    context_for_editing.to_owned(),
                    &request_id,
                )
                .await?;
            let original_code = &edited_code.original_code;
            let edited_code = &edited_code.edited_code;
            // debugging loop after this
            let _ = self
                .tools
                .check_code_correctness(
                    &sub_symbol_to_edit,
                    original_code,
                    edited_code,
                    &context_for_editing.join("\n"),
                    self.llm_properties.llm().clone(),
                    self.llm_properties.provider().clone(),
                    self.llm_properties.api_key().clone(),
                    request_id_ref,
                    &self.tool_properties,
                )
                .await;

            // once we have successfully changed the implementation over here
            // we have to start looking for followups over here
            // F in the chat for error handling :')
            let _ = self
                .tools
                .check_for_followups(
                    &sub_symbol_to_edit,
                    &original_code,
                    self.llm_properties.llm().clone(),
                    self.llm_properties.provider().clone(),
                    self.llm_properties.api_key().clone(),
                    self.hub_sender.clone(),
                    request_id_ref,
                    &self.tool_properties,
                )
                .await;
        }
        Ok(())
    }

    // this is the function which is polling the requests from the hub
    // we also want another loop which allows the agent to wait
    // for the requests which it was waiting for after sending it to the hub
    pub async fn run(
        self,
        receiver: UnboundedReceiver<(
            SymbolEvent,
            String,
            // we had a single sender over here as a future we can poll
            // for to receieve events from
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
    ) -> Result<(), SymbolError> {
        println!("Symbol::run({}) at types.rs", self.symbol_name());
        let receiver_stream = UnboundedReceiverStream::new(receiver);
        receiver_stream
            .map(|symbol_event| (symbol_event, self.clone()))
            .map(|(symbol_event, symbol)| async move {
                let (event, request_id, sender) = symbol_event;
                println!(
                    "Symbol::receiver_stream::event::({})\n{:?}",
                    symbol.symbol_name(),
                    &event
                );
                let _ = symbol.ui_sender.send(UIEventWithID::from_symbol_event(
                    request_id.to_owned(),
                    SymbolEventRequest::new(
                        symbol.symbol_identifier.clone(),
                        event.clone(),
                        symbol.tool_properties.clone(),
                    ),
                ));
                match event {
                    SymbolEvent::InitialRequest(initial_request) => {
                        println!("Symbol::inital_request: {}", symbol.symbol_name());
                        let initial_request = symbol
                            .generate_initial_request(
                                request_id.to_owned(),
                                initial_request.get_original_question(),
                            )
                            .await;
                        let request_sender = sender;
                        println!(
                            "Symbol::initial_request::generated({}).is_ok({})",
                            symbol.symbol_name(),
                            initial_request.is_ok(),
                        );
                        match initial_request {
                            Ok(initial_request) => {
                                let (sender, receiver) = tokio::sync::oneshot::channel();
                                let _ = symbol.hub_sender.send((
                                    initial_request,
                                    // since this is the initial request, we will end up generating
                                    // a new request id for this
                                    uuid::Uuid::new_v4().to_string(),
                                    sender,
                                ));
                                let response = receiver.await;
                                println!("Response from symbol.hub_sender: {:?}", &response);
                                // ideally we want to give this resopnse back to the symbol
                                // so it can keep track of everything that its doing, we will get to that
                                let _ = request_sender.send(SymbolEventResponse::TaskDone(
                                    "initial request done".to_owned(),
                                ));
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    SymbolEvent::Edit(edit_request) => {
                        // we refresh our state always
                        println!(
                            "symbol::types::symbol_event::edit::refresh_state({})",
                            symbol.symbol_name()
                        );
                        symbol.refresh_state(request_id.to_owned()).await;
                        // one of the primary goals here is that we can make edits
                        // everywhere at the same time unless its on the same file
                        // but for now, we are gonna pleb our way and make edits
                        // one by one
                        println!(
                            "symbol::types::symbol_event::edit::edit_implementations({})",
                            symbol.symbol_name()
                        );
                        symbol.edit_implementations(edit_request, request_id).await
                    }
                    SymbolEvent::AskQuestion(_ask_question_request) => {
                        // we refresh our state always
                        symbol.refresh_state(request_id).await;
                        // we will the following in sequence:
                        // - ask for information from surrounding nodes
                        // - refresh the state
                        // - ask for changes which need to be made to the surrounding nodes
                        // - refresh our state
                        // - edit ourselves if required or formulate the answer
                        // - followup
                        // - task 1: sending probes to the world about gathering information
                        todo!("ask question is not implemented yet");
                    }
                    SymbolEvent::Delete => {
                        todo!("delete is not implemented yet");
                    }
                    SymbolEvent::UserFeedback => {
                        todo!("user feedback is not implemented yet");
                    }
                    SymbolEvent::Outline => {
                        // we have been asked to provide an outline of the symbol we are part of
                        // this is a bit easy to do so lets try and finish this
                        let outline = symbol.get_outline(request_id).await;
                        let _ = match outline {
                            Ok(outline) => sender.send(SymbolEventResponse::TaskDone(outline)),
                            Err(_) => sender.send(SymbolEventResponse::TaskDone(
                                "failed to get outline".to_owned(),
                            )),
                        };
                        Ok(())
                    }
                    SymbolEvent::Probe(probe_request) => {
                        // we make the probe request an explicit request
                        // we are still going to do the same things just
                        // that this one is for gathering answeres
                        let reply = symbol
                            .probe_request(probe_request, symbol.hub_sender.clone(), request_id)
                            .await;
                        let _ = match reply {
                            Ok(reply) => sender.send(SymbolEventResponse::TaskDone(reply)),
                            Err(e) => {
                                println!("Error when probing: {:?}", e);
                                sender.send(SymbolEventResponse::TaskDone(
                                    "failed to look depeer to answer user query".to_owned(),
                                ))
                            }
                        };
                        Ok(())
                    }
                }
            })
            .buffer_unordered(1000)
            .collect::<Vec<_>>()
            .await;
        Ok(())
    }
}
