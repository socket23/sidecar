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
use llm_client::{
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    agentic::{
        symbol::{
            events::edit::{SymbolToEdit, SymbolToEditRequest},
            identifier::Snippet,
        },
        tool::lsp::open_file::OpenFileResponse,
    },
    chunking::{text_document::Range, types::OutlineNodeContent},
};

use super::{
    errors::SymbolError,
    events::types::SymbolEvent,
    identifier::{LLMProperties, MechaCodeSymbolThinking, SymbolIdentifier},
    tool_box::ToolBox,
};

#[derive(Debug, Clone)]
pub struct SymbolEventRequest {
    symbol: String,
    event: SymbolEvent,
}

pub struct SymbolEventResponse {}

/// The symbol is going to spin in the background and keep working on things
/// is this how we want it to work???
/// ideally yes, cause its its own process which will work in the background
#[derive(Derivative)]
#[derivative(PartialEq, Eq, Debug)]
pub struct Symbol {
    symbol_identifier: SymbolIdentifier,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    hub_sender: UnboundedSender<(
        SymbolEventRequest,
        tokio::sync::oneshot::Sender<SymbolEventResponse>,
    )>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    tools: Arc<ToolBox>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    mecha_code_symbol: MechaCodeSymbolThinking,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    llm_properties: LLMProperties,
}

impl Symbol {
    pub fn new(
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
    ) -> Self {
        Self {
            mecha_code_symbol,
            symbol_identifier,
            hub_sender,
            tools,
            llm_properties,
        }
    }

    // find the name of the sub-symbol
    pub fn find_subsymbol_in_range(&self, range: &Range, fs_file_path: &str) -> Option<String> {
        self.mecha_code_symbol
            .find_symbol_in_range(range, fs_file_path)
    }

    fn add_implementation_snippet(&mut self, snippet: Snippet) {
        self.mecha_code_symbol.add_implementation(snippet);
    }

    fn get_implementation_snippets(&self) -> &[Snippet] {
        self.mecha_code_symbol.get_implementations()
    }

    fn generate_ordered_snippets_list(&self, snippets: Vec<Snippet>) -> Vec<Snippet> {
        unimplemented!();
        // each symbol is also serializable and we can just ask the symbol broker
        // for the symbol at a position, since its also keeping track of the updates
    }

    // Loads the implementation after we do a go-to-implementations operation
    // in the ordered list from files to the symbols in an ordered fashion
    // this also allows for figuring out where to exactly make changes
    // in a very AST like way (cause thats the one which makes most sense
    // and is in-evitable)
    async fn setup_implementations(&mut self, snippets: Vec<Snippet>) {}

    /// Code selection logic for the symbols here
    async fn generate_initial_request(&mut self) -> Result<SymbolEvent, SymbolError> {
        let steps = self.mecha_code_symbol.steps().to_vec();
        if let Some(snippet) = self
            .mecha_code_symbol
            .get_snippet()
            .map(|snippet| snippet.clone())
        {
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
                implementation_content
                    .into_iter()
                    .for_each(|implementation_snippet| {
                        self.add_implementation_snippet(implementation_snippet);
                    });
            }

            // This is what we are trying to figure out
            // the idea representation here will be in the form of
            // now that we have added the snippets, we can ask the llm to rerank
            // the implementation snippets and figure out which to edit
            // once we have which to edit, we can then go to the references and keep
            // going from there whichever the LLM thinks is important for maintaining
            // the overall structure of the query
            // we also insert our own snipet into this
            // re-ranking for a complete symbol looks very different
            // we have to carefully craft the prompt in such a way that all the important
            // details are laid out properly
            // if its a class we call it a class, and if there are functions inside
            // it we call them out in a section, check how symbols are implemented
            // for a given LLM somewhere in the code
            // we have the text for all the snippets which are part of the class
            // there will be some here which will be the class definition and some
            // which are not part of it
            // so we use the ones which are part of the class defintion and name it
            // specially, so we can use it
            // struct A {....} is a special symbol
            // impl A {....} is also special and we show the symbols inside it one by
            // one for each function and in the order of they occur in the file
            // once we have the response we can set the agent to task on each of these snippets

            // TODO(skcd): We want to send this request for reranking
            // and get back the snippet indexes
            // and then we parse it back from here to get back to the symbol
            // we are interested in
            if let Some((ranked_xml_list, reverse_lookup)) = self.mecha_code_symbol.to_llm_request()
            {
                // now we send it over to the LLM and register as a rearank operation
                // and then ask the llm to reply back to us
                let filtered_list = self
                    .tools
                    .filter_code_snippets_in_symbol_for_editing(
                        ranked_xml_list,
                        steps.join("\n"),
                        self.llm_properties.llm().clone(),
                        self.llm_properties.provider().clone(),
                        self.llm_properties.api_key().clone(),
                    )
                    .await?;

                // now we take this filtered list and try to generate back and figure out
                // the ranges which need to be edited
                let code_to_edit_list = filtered_list.code_to_edit_list();
                // we use this to map it back to the symbols which we should
                // be editing and then send those are requests to the hub
                // which will forward it to the right symbol
                let sub_symbols_to_edit = reverse_lookup
                    .into_iter()
                    .filter_map(|reverse_lookup| {
                        let idx = reverse_lookup.idx();
                        let range = reverse_lookup.range();
                        let fs_file_path = reverse_lookup.fs_file_path();
                        let outline = reverse_lookup.is_outline();
                        let found_reason_to_edit = code_to_edit_list
                            .snippets()
                            .into_iter()
                            .find(|snippet| snippet.id() == idx)
                            .map(|snippet| snippet.reason_to_edit().to_owned());
                        match found_reason_to_edit {
                            Some(reason) => {
                                let symbol_in_range =
                                    self.find_subsymbol_in_range(range, fs_file_path);
                                // now we need to figure out how to edit it out
                                // properly
                                if let Some(symbol) = symbol_in_range {
                                    Some(SymbolToEdit::new(
                                        symbol,
                                        range.clone(),
                                        fs_file_path.to_owned(),
                                        vec![reason],
                                        outline,
                                    ))
                                } else {
                                    None
                                }
                            }
                            None => None,
                        }
                    })
                    .collect::<Vec<_>>();
                // TODO(skcd): Figure out what to do over here
                // when we are sending edit requests
                Ok(SymbolEvent::Edit(SymbolToEditRequest::new(
                    sub_symbols_to_edit,
                    self.symbol_identifier.clone(),
                )))
            } else {
                todo!("what do we do over here")
            }
        } else {
            // we have to figure out the location for this symbol and understand
            // where we want to put this symbol at
            // what would be the best way to do this?
            // should we give the folder overview and then ask it
            // or assume that its already written out
            todo!("figure out what to do here");
        }
    }

    pub async fn run(
        self,
        mut receiver: UnboundedReceiver<(
            SymbolEvent,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
    ) {
        // we can send the first request to the hub and then poll it from
        // the receiver over here
        // TODO(skcd): Pick it up from over here
        while let Some(event) = receiver.recv().await {
            // we are going to process the events one by one over here
            // we should also have a shut-down event which the symbol sends to itself
            // we can use the hub sender over here to make sure that forwarding the events
            // work as usual, its a roundabout way of doing it, but should work
            // TODO(skcd): Pick it up from here
        }
    }
}
