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

use crate::agentic::{symbol::identifier::Snippet, tool::lsp::open_file::OpenFileResponse};

use super::{
    errors::SymbolError,
    identifier::{MechaCodeSymbolThinking, SymbolIdentifier},
    tool_box::ToolBox,
};

#[derive(Debug, Clone)]
pub enum SymbolEvent {
    Create,
    AskQuestion,
    UserFeedback,
    Delete,
    Edit,
}

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
    ) -> Self {
        Self {
            mecha_code_symbol,
            symbol_identifier,
            hub_sender,
            tools,
        }
    }

    fn add_implementation_snippet(&mut self, snippet: Snippet) {
        self.mecha_code_symbol.add_implementation(snippet);
    }

    /// Code selection logic for the symbols here
    async fn generate_initial_request(&mut self) -> Result<MechaCodeSymbolThinking, SymbolError> {
        if let Some(snippet) = self.mecha_code_symbol.get_snippet() {
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
                .go_to_implementation(snippet, self.symbol_identifier.symbol_name())
                .await?;
            let unique_files = implementations
                .get_implementation_locations_vec()
                .iter()
                .map(|implementation| implementation.fs_file_path().to_owned())
                .collect::<HashSet<String>>();
            let cloned_tools = self.tools.clone();
            // once we have the unique files we have to request to open these locations
            let file_content_map = stream::iter(unique_files)
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
            // Once we have the file content map, we can read the ranges which we are
            // interested in and generate the implementation areas
            // we have to figure out how to handle updates etc as well, but we will get
            // to that later
            let implementation_content = implementations
                .get_implementation_locations_vec()
                .iter()
                .filter_map(|implementation| {
                    let file_path = implementation.fs_file_path().to_owned();
                    let range = implementation.range();
                    // if file content is empty, then we do not add this to our
                    // implementations
                    let file_content = file_content_map.get(&file_path);
                    if let Some(Ok(file_content)) = file_content {
                        if let Some(content) = file_content.content_in_range(&range) {
                            Some(Snippet::new(
                                self.symbol_identifier.symbol_name().to_owned(),
                                range.clone(),
                                file_path,
                                content,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            implementation_content
                .into_iter()
                .for_each(|implementation_snippet| {
                    self.add_implementation_snippet(implementation_snippet);
                });

            // now that we have added the snippets, we can ask the llm to rerank
            // the implementation snippets and figure out which to edit
            // once we have which to edit, we can then go to the references and keep
            // going from there whichever the LLM thinks is important for maintaining
            // the overall structure of the query
        } else {
            // we have to figure out the location for this symbol and understand
            // where we want to put this symbol at
            // what would be the best way to do this?
            // should we give the folder overview and then ask it
            // or assume that its already written out
        }
        unimplemented!();
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
