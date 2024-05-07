//! This is the most basic of the mechas
//! The way this will work is the following:
//! Each mecha will focus on a single coe symbol at all times, and then auxiliary
//! ones which we are depending on
//! This way we can pass more information about the symbols which are required etc
//! this will be useful for later

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::channel::mpsc::UnboundedSender;
use futures::lock::Mutex;

use crate::agentic::tool::base::Tool;
use crate::agentic::tool::code_symbol::important::CodeSymbolImportantResponse;
use crate::agentic::tool::errors::ToolError;
use crate::agentic::tool::grep::file::{FindInFileRequest, FindInFileResponse};
use crate::agentic::tool::input::ToolInput;
use crate::agentic::tool::lsp::gotodefintion::{GoToDefinitionRequest, GoToDefinitionResponse};
use crate::agentic::tool::lsp::open_file::{OpenFileRequest, OpenFileResponse};
use crate::agentic::tool::output::ToolOutput;
use crate::chunking::editor_parsing::EditorParsing;
use crate::chunking::text_document::Position;
use crate::chunking::types::{OutlineNode, OutlineNodeContent};
use crate::{
    agentic::tool::broker::ToolBroker, chunking::text_document::Range,
    inline_completion::symbols_tracker::SymbolTrackerInline,
};

use super::events::input::MechaInputEvent;

struct Snippet {
    range: Range,
    fs_file_path: String,
}

impl Snippet {
    pub fn new(range: Range, fs_file_path: String) -> Self {
        Self {
            range,
            fs_file_path,
        }
    }
}

struct MechaCodeSymbolThinking {
    symbol_name: String,
    steps: Vec<String>,
    is_new: bool,
    file_path: String,
    snippet: Option<Snippet>,
}

impl MechaCodeSymbolThinking {
    fn new(symbol_name: String, file_path: String) -> Self {
        Self {
            symbol_name,
            steps: Vec::new(),
            is_new: true,
            file_path,
            snippet: None,
        }
    }

    pub fn set_snippet(&mut self, snippet: Snippet) {
        self.snippet = Some(snippet);
    }
}

#[derive(Debug, Clone)]
enum MechaState {
    NoSymbol,
    Exploring,
    Editing,
    Fixing,
    Completed,
}

// What is the symbol we are focussing on
// this is important so we are able to work on a given symbol at a time, I do not
// know a better way to manage dependencies (this is the best and the most robotoic way
// of doing this)
struct MechaSymbol {
    symbol_name: String,
    range: Range,
    fs_file_path: String,
}

struct MechaMemory {
    snippets: Vec<Snippet>,
}

struct MechaContext {
    files: Vec<String>,
}

enum InputType {
    Snippet,
    Symbol,
    File,
    Folder,
}

// What are the events which invoke the meka, we first send an inital one from our
// side and then render it on the UI somehow
pub enum MechaEvent {
    InitialRequest(MechaInputEvent),
}

// Not entirely sure about how this will work, especially given that
// we might have cases where the mecha is just dicovering nodes or something
// something to figure out later on? for now we assume that there
// is a single node always attached to the mecha
// what happens when its a file and not a symbol
// TODO(skcd): How do we keep track of files over here which we need to make changes
// to, since that is also important
#[derive(Clone)]
pub struct SymbolLocking {
    symbols: Arc<Mutex<HashMap<String, UnboundedSender<MechaEvent>>>>,
}

impl SymbolLocking {
    pub fn new() -> Self {
        Self {
            symbols: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // checks if the symbol already exists, if it does then we are good
    pub async fn check_if_symbol_exists(&self, symbol: &str) -> bool {
        let mut symbols = self.symbols.lock().await;
        symbols.contains_key(symbol)
    }

    // this allows us to get back the symbol sender
    pub async fn get_symbol_sender(&self, symbol: &str) -> Option<UnboundedSender<MechaEvent>> {
        let symbols = self.symbols.lock().await;
        symbols.get(symbol).cloned()
    }

    // this tells us that we have a new sender avaiable
    pub async fn insert_symbol_sender(&self, symbol: &str, sender: UnboundedSender<MechaEvent>) {
        let mut symbols = self.symbols.lock().await;
        symbols.insert(symbol.to_string(), sender);
    }
}

pub struct MechaBasic {
    symbol_name: Option<MechaSymbol>,
    history_symbols: Vec<String>,
    current_query: Option<String>,
    // Lets keep it this way so we can pass a trace of all we have done
    interactions: Vec<String>,
    state: MechaState,
    tools: Arc<ToolBroker>,
    symbol_broker: Arc<SymbolTrackerInline>,
    editor_parsing: Arc<EditorParsing>,
    symbol_locking: SymbolLocking,
    editor_url: String,
    children_mechas: Vec<MechaBasic>,
}

impl MechaBasic {
    pub fn new(
        tools: Arc<ToolBroker>,
        symbol_broker: Arc<SymbolTrackerInline>,
        editor_parsing: Arc<EditorParsing>,
        symbol_locking: SymbolLocking,
        editor_url: String,
    ) -> Self {
        Self {
            symbol_name: None,
            history_symbols: Vec::new(),
            current_query: None,
            interactions: Vec::new(),
            state: MechaState::NoSymbol,
            tools,
            symbol_broker,
            editor_parsing,
            symbol_locking,
            editor_url,
            children_mechas: Vec::new(),
        }
    }

    // we need a function here which will just call tools and move between
    // states and maybe even spawn new mechas at some point, the goal is that
    // we only focus on a single symbol at a time
    // how do we go about designing that, lets start with a loop and see how well
    // we can do
    async fn get_tool_input_from_event(&mut self, event: MechaEvent) -> Option<ToolInput> {
        let state = self.state.clone();
        match state {
            MechaState::NoSymbol => {
                match event {
                    MechaEvent::InitialRequest(request) => request.tool_use_on_initial_invocation(),
                }
                // if we have no symbol then we should invoke the tool which
                // gives us back some data about the symbols which we should select and
                // focus on
                // we just invoke the initial exploration message along with all the context
                // we are passed with, this way we mutate our own state and invoke an action
            }
            MechaState::Exploring => {
                // we ask for the most important symbols here if we have no starting
                // point
                None
            }
            MechaState::Fixing => None,
            MechaState::Editing => None,
            MechaState::Completed => None,
        }
        // Now that we have the next tool use
        // we can invoke the tool using the tool broker
    }

    async fn invoke_tool_broker(&self, tool_input: ToolInput) -> Result<ToolOutput, ToolError> {
        self.tools.invoke(tool_input).await
    }

    // Now we have tha basic iteration loop setup, we know this is bad but this
    // is enough to get started
    // once we have the tool output, we act on it over here and wait for our
    // next iteration to start
    async fn invoke_tool(&mut self, event: MechaEvent) -> Result<ToolOutput, ToolError> {
        let tool = self.get_tool_input_from_event(event).await;
        if let Some(tool) = tool {
            self.invoke_tool_broker(tool).await
        } else {
            Err(ToolError::MissingTool)
        }
    }

    async fn find_in_file(
        &self,
        file_content: String,
        symbol: String,
    ) -> Result<FindInFileResponse, ToolError> {
        self.tools
            .invoke(ToolInput::GrepSingleFile(FindInFileRequest::new(
                file_content,
                symbol,
            )))
            .await
            .map(|result| result.grep_single_file())
            .map(|result| result.ok_or(ToolError::WrongToolInput))?
    }

    async fn file_open(&self, fs_file_path: String) -> Result<OpenFileResponse, ToolError> {
        self.tools
            .invoke(ToolInput::OpenFile(OpenFileRequest::new(
                fs_file_path,
                self.editor_url.to_owned(),
            )))
            .await
            .map(|result| result.get_file_open_response())
            .map(|result| result.ok_or(ToolError::WrongToolInput))?
    }

    async fn go_to_definition(
        &self,
        fs_file_path: &str,
        position: Position,
    ) -> Result<GoToDefinitionResponse, ToolError> {
        self.tools
            .invoke(ToolInput::GoToDefinition(GoToDefinitionRequest::new(
                fs_file_path.to_owned(),
                self.editor_url.to_owned(),
                position,
            )))
            .await
            .map(|result| result.get_go_to_definition())
            .map(|result| result.ok_or(ToolError::WrongToolInput))?
    }

    /// Grabs the symbol content and the range in the file which it is present in
    async fn grab_symbol_content_from_definition(
        &self,
        symbol_name: &str,
        definition: GoToDefinitionResponse,
    ) -> Result<Snippet, ToolError> {
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
                    Snippet::new(symbol.range().clone(), definition.file_path().to_owned())
                })
                .ok_or(ToolError::SymbolNotFound(symbol_name.to_owned()))
        } else {
            Err(ToolError::SymbolNotFound(symbol_name.to_owned()))
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
    async fn important_symbols(
        &mut self,
        important_symbols: CodeSymbolImportantResponse,
    ) -> Result<Vec<MechaCodeSymbolThinking>, ToolError> {
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
            let _ = self.file_open(code_snippet.file_path.to_owned()).await;

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
                    println!("this is pretty bad, read the comment above on what is happening");
                }
            }

            mecha_symbols.push(code_snippet);
        }
        Ok(mecha_symbols)
    }

    pub async fn iterate(&mut self, event: MechaEvent) -> Result<Vec<MechaEvent>, ToolError> {
        let output = self.invoke_tool(event).await?;
        // Now we try to take the next action based on the output
        let next_executions = match output {
            ToolOutput::CodeEditTool(code_editing) => {}
            ToolOutput::ImportantSymbols(important_symbols) => {
                match self.state {
                    MechaState::NoSymbol => {
                        // if we have no symbols to keep track of, we can just start a few other
                        // mechas running in the background and then try to send events to them
                        // since they will be running and all and want to communicate properly
                        println!("{:?}", important_symbols);
                        // how do we create a new mecha here?
                        let important_symbols = self.important_symbols(important_symbols).await?;
                        // we can create a new mecha here and start the process again
                    }
                    MechaState::Exploring => {
                        // we need to select the most important symbol here
                        // and then we need to move to the exploring state
                        // we also need to update our history symbols
                        // we also need to update our current query
                        // we also need to update our interactions
                        // we also need to update our state
                    }
                    MechaState::Fixing => {}
                    MechaState::Editing => {}
                    MechaState::Completed => {}
                }
            }
            ToolOutput::CodeToEdit(code_to_edit) => {}
            ToolOutput::ReRankSnippets(rerank_snippets) => {}
            ToolOutput::LSPDiagnostics(lsp_diagnostics) => {}
            ToolOutput::GoToDefinition(_) => {}
            ToolOutput::FileOpen(_) => {}
            ToolOutput::GrepSingleFile(_) => {}
        };
        todo!();
    }
}
