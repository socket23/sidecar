//! We are going to focus on a single symbol and have attributes for it
//! Some of the attributes here might be related to the symbol itself
//! or the general question which is being asked to the symbol

use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};

use derivative::Derivative;
use futures::lock::Mutex;
use futures::{stream, StreamExt};
use thiserror::Error;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::agentic::tool::base::Tool;
use crate::agentic::tool::broker::ToolBroker;
use crate::agentic::tool::code_symbol::important::CodeSymbolImportantResponse;
use crate::agentic::tool::errors::ToolError;
use crate::agentic::tool::grep::file::{FindInFileRequest, FindInFileResponse};
use crate::agentic::tool::input::ToolInput;
use crate::agentic::tool::lsp::gotodefintion::{GoToDefinitionRequest, GoToDefinitionResponse};
use crate::agentic::tool::lsp::gotoimplementations::{
    GoToImplementationRequest, GoToImplementationResponse,
};
use crate::agentic::tool::lsp::open_file::{OpenFileRequest, OpenFileResponse};
use crate::agentic::tool::output::ToolOutput;
use crate::chunking::text_document::{Position, Range};
use crate::chunking::types::{OutlineNode, OutlineNodeContent};
use crate::inline_completion::symbols_tracker::SymbolTrackerInline;

use super::events::input::MechaInputEvent;

// we have a central symbol manager
// where all the events from the symbols (whichever symbol they want to talk to go)
// then a symbol receives the signal and then we act on it
// we might want to talk to other symbols or ask them to change
// to accomplish this the symbol we are talking to will respond back to us after
// we have recieved a signal back from it that the question we asked it was answered
// we talk to the central symbol manager and await on the reply from the symbol manager
// that's how this whole system should work out

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SymbolIdentifier {
    symbol_name: String,
    fs_file_path: Option<String>,
}

impl SymbolIdentifier {
    pub fn new_symbol(symbol_name: &str) -> Self {
        Self {
            symbol_name: symbol_name.to_owned(),
            fs_file_path: None,
        }
    }

    pub fn with_file_path(symbol_name: &str, fs_file_path: &str) -> Self {
        Self {
            symbol_name: symbol_name.to_owned(),
            fs_file_path: Some(fs_file_path.to_owned()),
        }
    }
}

#[derive(Debug)]
pub struct MechaCodeSymbolThinking {
    symbol_name: String,
    steps: Vec<String>,
    is_new: bool,
    file_path: String,
    snippet: Option<Snippet>,
    implementations: Vec<Snippet>,
}

impl MechaCodeSymbolThinking {
    fn new(symbol_name: String, file_path: String) -> Self {
        Self {
            symbol_name,
            steps: vec![],
            is_new: true,
            file_path,
            snippet: None,
            implementations: vec![],
        }
    }

    fn to_symbol_identifier(&self) -> SymbolIdentifier {
        if self.is_new {
            SymbolIdentifier::new_symbol(&self.symbol_name)
        } else {
            SymbolIdentifier::with_file_path(&self.symbol_name, &self.file_path)
        }
    }

    pub fn set_snippet(&mut self, snippet: Snippet) {
        self.snippet = Some(snippet);
    }
}

#[derive(Debug, Error)]
pub enum SymbolError {
    #[error("Tool error: {0}")]
    ToolError(ToolError),

    #[error("Wrong tool output")]
    WrongToolOutput,
}

pub enum SymbolEvent {
    Create,
    AskQuestion,
    UserFeedback,
    Delete,
    Edit,
}

pub struct SymbolEventRequest {
    symbol: String,
    event: SymbolEvent,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Snippet {
    range: Range,
    symbol_name: String,
    fs_file_path: String,
}

impl Snippet {
    pub fn new(symbol_name: String, range: Range, fs_file_path: String) -> Self {
        Self {
            symbol_name,
            range,
            fs_file_path,
        }
    }
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
    tools: Arc<ToolBroker>,
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
        tools: Arc<ToolBroker>,
    ) -> Self {
        Self {
            mecha_code_symbol,
            symbol_identifier,
            hub_sender,
            tools,
        }
    }

    async fn run(
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

#[derive(Clone)]
struct SymbolLocker {
    symbols: Arc<
        Mutex<
            HashMap<
                // TODO(skcd): what should be the key here for this to work properly
                // cause we can have multiple symbols which share the same name
                // this probably would not happen today but would be good to figure
                // out at some point
                SymbolIdentifier,
                // this is the channel which we use to talk to this particular symbol
                // and everything related to it
                UnboundedSender<(
                    SymbolEvent,
                    tokio::sync::oneshot::Sender<SymbolEventResponse>,
                )>,
            >,
        >,
    >,
    // this is the main communication channel which we can use to send requests
    // to the right symbol
    hub_sender: UnboundedSender<(
        SymbolEventRequest,
        tokio::sync::oneshot::Sender<SymbolEventResponse>,
    )>,
    tools: Arc<ToolBroker>,
}

impl SymbolLocker {
    pub fn new(
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
        tools: Arc<ToolBroker>,
    ) -> Self {
        Self {
            symbols: Arc::new(Mutex::new(HashMap::new())),
            hub_sender,
            tools,
        }
    }

    async fn process_request(
        &self,
        request_event: (
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        ),
    ) {
        let request = request_event.0;
        let sender = request_event.1;
        // we will send the response in this sender
    }

    async fn create_symbol_agent(&self, request: MechaCodeSymbolThinking) {
        // say we create the symbol agent, what happens next
        // the agent can have its own events which it might need to do, including the
        // followups or anything else
        // the user might have some events to send
        // other agents might also want to talk to it for some information
        let symbol_identifier = request.to_symbol_identifier();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<(
            SymbolEvent,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>();
        {
            let mut symbols = self.symbols.lock().await;
            symbols.insert(symbol_identifier.clone(), sender);
        }

        // now we create the symbol and let it rip
        let symbol = Symbol::new(
            symbol_identifier,
            request,
            self.hub_sender.clone(),
            self.tools.clone(),
        );

        // now we let it rip, we give the symbol the receiver and ask it
        // to go crazy with it
        tokio::spawn(async move { symbol.run(receiver).await });
        // fin
    }
}

// This is the main communication manager between all the symbols
// this of this as the central hub through which all the events go forward
pub struct SymbolManager {
    sender: UnboundedSender<(
        SymbolEventRequest,
        tokio::sync::oneshot::Sender<SymbolEventResponse>,
    )>,
    // this is the channel where the various symbols will use to talk to the manager
    // which in turn will proxy it to the right symbol, what happens if there are failures
    // each symbol has its own receiver which is being used
    symbol_locker: SymbolLocker,
    tools: Arc<ToolBroker>,
    symbol_broker: Arc<SymbolTrackerInline>,
    editor_url: String,
}

impl SymbolManager {
    pub fn new(
        tools: Arc<ToolBroker>,
        symbol_broker: Arc<SymbolTrackerInline>,
        editor_url: String,
    ) -> Self {
        let (sender, mut receier) = tokio::sync::mpsc::unbounded_channel::<(
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>();
        let symbol_locker = SymbolLocker::new(sender.clone(), tools.clone());
        let cloned_symbol_locker = symbol_locker.clone();
        tokio::spawn(async move {
            // TODO(skcd): Make this run in full parallelism in the future, for
            // now this is fine
            while let Some(event) = receier.recv().await {
                let _ = cloned_symbol_locker.process_request(event).await;
            }
        });
        Self {
            sender,
            symbol_locker,
            tools,
            symbol_broker,
            editor_url,
        }
    }

    // once we have the initial request, which we will go through the initial request
    // mode once, we have the symbols from it we can use them to spin up sub-symbols as well
    pub async fn initial_request(&self, input_event: MechaInputEvent) -> Result<(), SymbolError> {
        let tool_input = input_event.tool_use_on_initial_invocation();
        if let Some(tool_input) = tool_input {
            if let ToolOutput::ImportantSymbols(important_symbols) = self
                .tools
                .invoke(tool_input)
                .await
                .map_err(|e| SymbolError::ToolError(e))?
            {
                let symbols = self
                    .important_symbols(important_symbols)
                    .await
                    .map_err(|e| e.into())?;
                // This is where we are creating all the symbols
                let _ = stream::iter(symbols)
                    .map(|symbol_request| async move {
                        let _ = self.symbol_locker.create_symbol_agent(symbol_request);
                    })
                    .buffer_unordered(100)
                    .collect::<Vec<_>>()
                    .await;
            }
        } else {
            // We are for some reason not even invoking the first passage which is
            // weird, this is completely wrong and we should be replying back
            println!("this is wrong, please look at the comment here");
        }
        Ok(())
    }

    async fn invoke_tool_broker(&self, tool_input: ToolInput) -> Result<ToolOutput, ToolError> {
        self.tools.invoke(tool_input).await
    }

    async fn go_to_implementation(
        &self,
        snippet: &Snippet,
        symbol_name: &str,
    ) -> Result<GoToImplementationResponse, SymbolError> {
        // LSP requies the EXACT symbol location on where to click go-to-implementation
        // since thats the case we can just open the file and then look for the
        // first occurance of the symbol and grab the location
        let file_content = self.file_open(snippet.fs_file_path.to_owned()).await?;
        let find_in_file = self
            .find_in_file(file_content.contents(), symbol_name.to_owned())
            .await?;
        if let Some(position) = find_in_file.get_position() {
            self.tools
                .invoke(ToolInput::SymbolImplementations(
                    GoToImplementationRequest::new(
                        snippet.fs_file_path.to_owned(),
                        position,
                        self.editor_url.to_owned(),
                    ),
                ))
                .await
                .map_err(|e| SymbolError::ToolError(e))?
                .get_go_to_implementation()
                .ok_or(SymbolError::WrongToolOutput)
        } else {
            Err(SymbolError::ToolError(ToolError::SymbolNotFound(
                symbol_name.to_owned(),
            )))
        }
    }

    async fn find_in_file(
        &self,
        file_content: String,
        symbol: String,
    ) -> Result<FindInFileResponse, SymbolError> {
        self.tools
            .invoke(ToolInput::GrepSingleFile(FindInFileRequest::new(
                file_content,
                symbol,
            )))
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .grep_single_file()
            .ok_or(SymbolError::WrongToolOutput)
    }

    async fn file_open(&self, fs_file_path: String) -> Result<OpenFileResponse, SymbolError> {
        self.tools
            .invoke(ToolInput::OpenFile(OpenFileRequest::new(
                fs_file_path,
                self.editor_url.to_owned(),
            )))
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_file_open_response()
            .ok_or(SymbolError::WrongToolOutput)
    }

    async fn go_to_definition(
        &self,
        fs_file_path: &str,
        position: Position,
    ) -> Result<GoToDefinitionResponse, SymbolError> {
        self.tools
            .invoke(ToolInput::GoToDefinition(GoToDefinitionRequest::new(
                fs_file_path.to_owned(),
                self.editor_url.to_owned(),
                position,
            )))
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_go_to_definition()
            .ok_or(SymbolError::WrongToolOutput)
    }

    /// Grabs the symbol content and the range in the file which it is present in
    async fn grab_symbol_content_from_definition(
        &self,
        symbol_name: &str,
        definition: GoToDefinitionResponse,
    ) -> Result<Snippet, SymbolError> {
        // here we first try to open the file
        // and then read the symbols from it nad then parse
        // it out properly
        // since its very much possible that we get multiple definitions over here
        // we have to figure out how to pick the best one over here
        // TODO(skcd): This will break if we are unable to get definitions properly
        let definition = definition.definitions().remove(0);
        let _ = self.file_open(definition.file_path().to_owned()).await?;
        // grab the symbols from the file
        // but we can also try getting it from the symbol broker
        // because we are going to open a file and send a signal to the signal broker
        // let symbols = self
        //     .editor_parsing
        //     .for_file_path(definition.file_path())
        //     .ok_or(ToolError::NotSupportedLanguage)?
        //     .generate_file_outline_str(file_content.contents().as_bytes());
        let symbols = self
            .symbol_broker
            .get_symbols_outline(definition.file_path())
            .await;
        if let Some(symbols) = symbols {
            let symbols = self.grab_symbols_from_outline(symbols, symbol_name);
            // find the first symbol and grab back its content
            symbols
                .iter()
                .find(|symbol| symbol.name() == symbol_name)
                .map(|symbol| {
                    Snippet::new(
                        symbol.name().to_owned(),
                        symbol.range().clone(),
                        definition.file_path().to_owned(),
                    )
                })
                .ok_or(SymbolError::ToolError(ToolError::SymbolNotFound(
                    symbol_name.to_owned(),
                )))
        } else {
            Err(SymbolError::ToolError(ToolError::SymbolNotFound(
                symbol_name.to_owned(),
            )))
        }
    }

    fn grab_symbols_from_outline(
        &self,
        outline_nodes: Vec<OutlineNode>,
        symbol_name: &str,
    ) -> Vec<OutlineNodeContent> {
        outline_nodes
            .into_iter()
            .filter_map(|node| {
                if node.is_class() {
                    // it might either be the class itself
                    // or a function inside it so we can check for it
                    // properly here
                    if node.content().name() == symbol_name {
                        Some(vec![node.content().clone()])
                    } else {
                        Some(
                            node.children()
                                .into_iter()
                                .filter(|node| node.name() == symbol_name)
                                .map(|node| node.clone())
                                .collect::<Vec<_>>(),
                        )
                    }
                } else {
                    // we can just compare the node directly
                    // without looking at the children at this stage
                    if node.content().name() == symbol_name {
                        Some(vec![node.content().clone()])
                    } else {
                        None
                    }
                }
            })
            .flatten()
            .collect::<Vec<_>>()
    }

    // TODO(skcd): Improve this since we have code symbols which might be duplicated
    // because there can be repetitions and we can'nt be sure where they exist
    // one key hack here is that we can legit search for this symbol and get
    // to the definition of this very easily
    pub async fn important_symbols(
        &self,
        important_symbols: CodeSymbolImportantResponse,
    ) -> Result<Vec<MechaCodeSymbolThinking>, SymbolError> {
        let symbols = important_symbols.symbols();
        let ordered_symbols = important_symbols.ordered_symbols();
        // there can be overlaps between these, but for now its fine
        let mut new_symbols: HashSet<String> = Default::default();
        let mut symbols_to_visit: HashSet<String> = Default::default();
        let mut final_code_snippets: HashMap<String, MechaCodeSymbolThinking> = Default::default();
        ordered_symbols.iter().for_each(|ordered_symbol| {
            let code_symbol = ordered_symbol.code_symbol().to_owned();
            if ordered_symbol.is_new() {
                new_symbols.insert(code_symbol.to_owned());
                final_code_snippets.insert(
                    code_symbol.to_owned(),
                    MechaCodeSymbolThinking {
                        symbol_name: code_symbol,
                        steps: ordered_symbol.steps().to_owned(),
                        is_new: true,
                        file_path: ordered_symbol.file_path().to_owned(),
                        snippet: None,
                        implementations: vec![],
                    },
                );
            } else {
                symbols_to_visit.insert(code_symbol.to_owned());
                final_code_snippets.insert(
                    code_symbol.to_owned(),
                    MechaCodeSymbolThinking {
                        symbol_name: code_symbol,
                        steps: ordered_symbol.steps().to_owned(),
                        is_new: false,
                        file_path: ordered_symbol.file_path().to_owned(),
                        snippet: None,
                        implementations: vec![],
                    },
                );
            }
        });
        symbols.iter().for_each(|symbol| {
            // if we do not have the new symbols being tracked here, we use it
            // for exploration
            if !new_symbols.contains(symbol.code_symbol()) {
                symbols_to_visit.insert(symbol.code_symbol().to_owned());
                if let Some(code_snippet) = final_code_snippets.get_mut(symbol.code_symbol()) {
                    code_snippet.steps.push(symbol.thinking().to_owned());
                }
            }
        });

        let mut mecha_symbols = vec![];

        for (_, mut code_snippet) in final_code_snippets.into_iter() {
            // we always open the document before asking for an outline
            let file_open_result = self.file_open(code_snippet.file_path.to_owned()).await?;
            println!("{:?}", file_open_result);
            let language = file_open_result.language().to_owned();
            // we add the document for parsing over here
            self.symbol_broker
                .add_document(
                    file_open_result.fs_file_path().to_owned(),
                    file_open_result.contents(),
                    language,
                )
                .await;

            // we grab the outlines over here
            let outline_nodes = self
                .symbol_broker
                .get_symbols_outline(&code_snippet.file_path)
                .await;

            // We will either get an outline node or we will get None
            // for today, we will go with the following assumption
            // - if the document has already been open, then its good
            // - otherwise we open the document and parse it again
            if let Some(outline_nodes) = outline_nodes {
                let mut outline_nodes =
                    self.grab_symbols_from_outline(outline_nodes, &code_snippet.symbol_name);

                // if there are no outline nodes, then we have to skip this part
                // and keep going
                if outline_nodes.is_empty() {
                    // here we need to do go-to-definition
                    // first we check where the symbol is present on the file
                    // and we can use goto-definition
                    // so we first search the file for where the symbol is
                    // this will be another invocation to the tools
                    // and then we ask for the definition once we find it
                    let file_data = self.file_open(code_snippet.file_path.to_owned()).await?;
                    let file_content = file_data.contents();
                    // now we parse it and grab the outline nodes
                    let find_in_file = self
                        .find_in_file(file_content, code_snippet.symbol_name.to_owned())
                        .await
                        .map(|find_in_file| find_in_file.get_position())
                        .ok()
                        .flatten();
                    // now that we have a poition, we can ask for go-to-definition
                    if let Some(file_position) = find_in_file {
                        let definition = self
                            .go_to_definition(&code_snippet.file_path, file_position)
                            .await?;
                        // let definition_file_path = definition.file_path().to_owned();
                        let snippet_node = self
                            .grab_symbol_content_from_definition(
                                &code_snippet.symbol_name,
                                definition,
                            )
                            .await?;
                        code_snippet.set_snippet(snippet_node);
                    }
                } else {
                    // if we have multiple outline nodes, then we need to select
                    // the best one, this will require another invocation from the LLM
                    // we have the symbol, we can just use the outline nodes which is
                    // the first
                    let outline_node = outline_nodes.remove(0);
                    code_snippet.set_snippet(Snippet::new(
                        outline_node.name().to_owned(),
                        outline_node.range().clone(),
                        outline_node.fs_file_path().to_owned(),
                    ));
                }
            } else {
                // if this is new, then we probably do not have a file path
                // to write it to
                if !code_snippet.is_new {
                    // its a symbol but we have nothing about it, so we log
                    // this as error for now, but later we have to figure out
                    // what to do about it
                    println!(
                        "this is pretty bad, read the comment above on what is happening {:?}",
                        &code_snippet.symbol_name
                    );
                }
            }

            mecha_symbols.push(code_snippet);
        }
        Ok(mecha_symbols)
    }
}
