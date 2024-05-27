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

use crate::{
    agentic::{
        symbol::{events::edit::SymbolToEditRequest, identifier::Snippet},
        tool::lsp::open_file::OpenFileResponse,
    },
    chunking::{text_document::Range, types::OutlineNodeContent},
};

use super::{
    errors::SymbolError,
    events::{
        edit::SymbolToEdit,
        probe::SymbolToProbeRequest,
        types::{AskQuestionRequest, SymbolEvent},
    },
    identifier::{LLMProperties, MechaCodeSymbolThinking, SymbolIdentifier},
    tool_box::ToolBox,
};

#[derive(Debug, Clone)]
pub struct SymbolEventRequest {
    symbol: SymbolIdentifier,
    event: SymbolEvent,
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
}

impl SymbolEventRequest {
    pub fn new(symbol: SymbolIdentifier, event: SymbolEvent) -> Self {
        Self { symbol, event }
    }

    pub fn initial_request(symbol: SymbolIdentifier) -> Self {
        Self {
            symbol,
            event: SymbolEvent::InitialRequest,
        }
    }

    pub fn outline(symbol: SymbolIdentifier) -> Self {
        Self {
            symbol,
            event: SymbolEvent::Outline,
        }
    }

    pub fn ask_question(symbol: SymbolIdentifier, question: String) -> Self {
        Self {
            symbol,
            event: SymbolEvent::AskQuestion(AskQuestionRequest::new(question)),
        }
    }
}

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
}

impl Symbol {
    pub async fn new(
        symbol_identifier: SymbolIdentifier,
        mecha_code_symbol: MechaCodeSymbolThinking,
        // this can be used to talk to other symbols and get them
        // to act on certain things
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
        tools: Arc<ToolBox>,
        llm_properties: LLMProperties,
    ) -> Result<Self, SymbolError> {
        let mut symbol = Self {
            mecha_code_symbol: Arc::new(mecha_code_symbol),
            symbol_identifier,
            hub_sender,
            tools,
            llm_properties,
        };
        // grab the implementations of the symbol
        // TODO(skcd): We also have to grab the diagnostics and auto-start any
        // process which we might want to
        symbol.grab_implementations().await?;
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

    async fn get_outline(&self) -> Result<String, SymbolError> {
        // to grab the outline first we need to understand the definition snippet
        // of the node and then create it appropriately
        // First thing we want to do here is to find the symbols which are present
        // in the file and get the one which corresponds to us, once we have that
        // we go to all the implementations and grab them as well and generate
        // the outline which is required
        self.tools
            .outline_nodes_for_symbol(self.fs_file_path(), self.symbol_name())
            .await
    }

    async fn grab_implementations(&self) -> Result<(), SymbolError> {
        let snippet: Option<Snippet>;
        {
            snippet = self.mecha_code_symbol.get_snippet().await.clone();
        }
        if let Some(snippet) = snippet {
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
                .go_to_implementation(&snippet, self.symbol_identifier.symbol_name())
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
                    let file_path = file_path.clone();
                    let file_content = tool_box.file_open(file_path.clone()).await;
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
                                        .filter(|outline_node| outline_node.range().contains(range))
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
    async fn probe_request(&self, request: SymbolToProbeRequest) -> Result<(), SymbolError> {
        // First we refresh our state over here
        self.refresh_state().await;

        let history = request.history();
        let query = request.probe_request();

        let snippets = self.mecha_code_symbol.get_implementations().await;
        // - sub-symbol selection for probing
        let sub_symbol_request = self
            .tools
            .probe_sub_symbols(
                snippets,
                &request,
                self.llm_properties.llm().clone(),
                self.llm_properties.provider().clone(),
                self.llm_properties.api_key().clone(),
            )
            .await?;
        // - ask if we should probe the sub-symbols here
        stream::iter(
            sub_symbol_request
                .snippets_to_probe_ordered()
                .into_iter()
                .map(|snippet_with_reason| {
                    let reason = snippet_with_reason.reason().to_owned();
                    let snippet = snippet_with_reason.remove_snippet();
                    (reason, snippet, self.llm_properties.clone())
                }),
        )
        .map(|(reason, snippet, llm_properties)| async move {
            // Now depending on the response here we can exlcude/include
            // the symbols which we want to follow and ask for more information
            let response = self
                .tools
                .should_follow_subsymbol_for_probing(
                    &snippet,
                    &reason,
                    history,
                    query,
                    llm_properties.llm().clone(),
                    llm_properties.provider().clone(),
                    llm_properties.api_key().clone(),
                )
                .await;
        })
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;
        // - ask the sub-symbol the probing question
        // - wait for the reply and then return the answer

        // Next we grab the important definitions which we are interested in
        // let definitions = self.tools.gather_important_symbols_with_definition(fs_file_path, file_content, selection_range, llm, provider, api_keys, query, hub_sender)
        Ok(())
    }

    /// Refreshing the state here implies the following:
    /// - we figure out all the implementations again and also
    /// our core snippet again
    /// - this way even if there have been changes we are almost always
    /// correct
    async fn refresh_state(&self) {
        // do we really have to do this? or can we get away from this just by
        // not worrying about things?
        let snippet = self
            .tools
            .find_snippet_for_symbol(self.fs_file_path(), self.symbol_name())
            .await;
        // if we do have a snippet here which is present update it, otherwise its a pretty
        // bad sign that we had the snippet before but do not have it now
        if let Ok(snippet) = snippet {
            self.mecha_code_symbol.set_snippet(snippet).await;
        }
        // now grab the implementations again
        let _ = self.grab_implementations().await;
    }

    async fn generate_initial_request(&self) -> Result<SymbolEventRequest, SymbolError> {
        // this is a very big block because of the LLM request, but lets see how
        // this plays out in practice
        self.mecha_code_symbol
            .initial_request(self.tools.clone(), self.llm_properties.clone())
            .await
    }

    // The protocol here is that the questions are just plain text, its on the symbol
    // to decide if it needs to collect more information or make changes, we need to carefully
    // figure that out over here
    // what tools do we provide to the symbol for this?
    async fn answer_question(&self, question: &str) -> Result<SymbolEventRequest, SymbolError> {
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
    async fn grab_context_for_editing(
        &self,
        subsymbol: &SymbolToEdit,
    ) -> Result<Vec<String>, SymbolError> {
        let file_content = self
            .tools
            .get_file_content(&subsymbol.fs_file_path())
            .await?;
        let symbol_to_edit = self.tools.find_symbol_to_edit(subsymbol).await?;
        let selection_range = symbol_to_edit.range();
        let language = self
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
            )
            .await?;
        let codebase_wide_search = self
            .tools
            .utlity_symbols_search(
                &subsymbol.instructions().join("\n"),
                interested_defintiions
                    .iter()
                    .filter_map(|interested_symbol| {
                        if let Some((code_symbol, _)) = interested_symbol {
                            Some(code_symbol)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
                &symbol_to_edit,
                &file_content,
                &subsymbol.fs_file_path(),
                self.mecha_code_symbol.user_context(),
                &language,
                self.llm_properties.llm().clone(),
                self.llm_properties.provider().clone(),
                self.llm_properties.api_key().clone(),
                self.hub_sender.clone(),
            )
            .await?;

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
    ) -> Result<EditedCodeSymbol, SymbolError> {
        let file_content = self
            .tools
            .get_file_content(&sub_symbol.fs_file_path())
            .await?;
        let symbol_to_edit = self.tools.find_symbol_to_edit(sub_symbol).await?;
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
    ) -> Result<(), SymbolError> {
        // here we might want to edit ourselves or generate new code depending
        // on the scope of the changes being made
        let sub_symbols_to_edit = edit_request.symbols();
        // edit requires the following:
        // - gathering context for the symbols which the definitions or outlines are required
        // - making the edits
        // - following the changed symbol to check on the references and wherever its being used
        for sub_symbol_to_edit in sub_symbols_to_edit.into_iter() {
            let context_for_editing = self.grab_context_for_editing(&sub_symbol_to_edit).await?;
            // always return the original code which was present here in case of rollbacks
            let edited_code = self
                .edit_code(&sub_symbol_to_edit, context_for_editing.to_owned())
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
            // we had a single sender over here as a future we can poll
            // for to receieve events from
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
    ) -> Result<(), SymbolError> {
        let receiver_stream = UnboundedReceiverStream::new(receiver);
        receiver_stream
            .map(|symbol_event| (symbol_event, self.clone()))
            .map(|(symbol_event, symbol)| async move {
                let (event, sender) = symbol_event;
                match event {
                    SymbolEvent::InitialRequest => {
                        let initial_request = symbol.generate_initial_request().await;
                        let _ = sender.send(SymbolEventResponse::TaskDone(
                            "initial list of symbols found".to_owned(),
                        ));
                        match initial_request {
                            Ok(initial_request) => {
                                let (sender, receiver) = tokio::sync::oneshot::channel();
                                let _ = symbol.hub_sender.send((initial_request, sender));
                                // ideally we want to give this resopnse back to the symbol
                                // so it can keep track of everything that its doing, we will get to that
                                let _response = receiver.await;

                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    SymbolEvent::Edit(edit_request) => {
                        // we refresh our state always
                        symbol.refresh_state().await;
                        // one of the primary goals here is that we can make edits
                        // everywhere at the same time unless its on the same file
                        // but for now, we are gonna pleb our way and make edits
                        // one by one
                        symbol.edit_implementations(edit_request).await
                    }
                    SymbolEvent::AskQuestion(ask_question_request) => {
                        // we refresh our state always
                        symbol.refresh_state().await;
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
                        let outline = symbol.get_outline().await;
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
                        let _ = symbol.probe_request(probe_request).await;
                        todo!("we need to implement this")
                    }
                }
            })
            .buffer_unordered(1000)
            .collect::<Vec<_>>()
            .await;
        Ok(())
    }
}
