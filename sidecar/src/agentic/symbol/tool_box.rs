use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use llm_client::clients::types::LLMType;
use llm_client::provider::{LLMProvider, LLMProviderAPIKeys};
use llm_prompts::reranking::types::ReRankCodeSpanResponse;
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::symbol::identifier::Snippet;
use crate::agentic::tool::base::Tool;
use crate::agentic::tool::code_symbol::important::CodeSymbolImportantResponse;
use crate::agentic::tool::errors::ToolError;
use crate::agentic::tool::grep::file::{FindInFileRequest, FindInFileResponse};
use crate::agentic::tool::lsp::gotodefintion::{GoToDefinitionRequest, GoToDefinitionResponse};
use crate::agentic::tool::lsp::gotoimplementations::{
    GoToImplementationRequest, GoToImplementationResponse,
};
use crate::agentic::tool::lsp::open_file::OpenFileResponse;
use crate::agentic::tool::rerank::base::{ReRankEntriesForBroker, ReRankRequestMetadata};
use crate::chunking::text_document::Position;
use crate::chunking::types::{OutlineNode, OutlineNodeContent};
use crate::{
    agentic::tool::{broker::ToolBroker, input::ToolInput, lsp::open_file::OpenFileRequest},
    inline_completion::symbols_tracker::SymbolTrackerInline,
};

use super::errors::SymbolError;
use super::identifier::MechaCodeSymbolThinking;
use super::ui_event::UIEvent;

#[derive(Clone)]
pub struct ToolBox {
    tools: Arc<ToolBroker>,
    symbol_broker: Arc<SymbolTrackerInline>,
    editor_url: String,
    ui_events: UnboundedSender<UIEvent>,
}

impl ToolBox {
    pub fn new(
        tools: Arc<ToolBroker>,
        symbol_broker: Arc<SymbolTrackerInline>,
        editor_url: String,
        ui_events: UnboundedSender<UIEvent>,
    ) -> Self {
        Self {
            tools,
            symbol_broker,
            editor_url,
            ui_events,
        }
    }

    pub async fn file_open(&self, fs_file_path: String) -> Result<OpenFileResponse, SymbolError> {
        let request = ToolInput::OpenFile(OpenFileRequest::new(
            fs_file_path,
            self.editor_url.to_owned(),
        ));
        let _ = self.ui_events.send(UIEvent::from(request.clone()));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_file_open_response()
            .ok_or(SymbolError::WrongToolOutput)
    }

    async fn find_in_file(
        &self,
        file_content: String,
        symbol: String,
    ) -> Result<FindInFileResponse, SymbolError> {
        let request = ToolInput::GrepSingleFile(FindInFileRequest::new(file_content, symbol));
        let _ = self.ui_events.send(UIEvent::from(request.clone()));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .grep_single_file()
            .ok_or(SymbolError::WrongToolOutput)
    }

    async fn go_to_definition(
        &self,
        fs_file_path: &str,
        position: Position,
    ) -> Result<GoToDefinitionResponse, SymbolError> {
        let request = ToolInput::GoToDefinition(GoToDefinitionRequest::new(
            fs_file_path.to_owned(),
            self.editor_url.to_owned(),
            position,
        ));
        let _ = self.ui_events.send(UIEvent::from(request.clone()));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_go_to_definition()
            .ok_or(SymbolError::WrongToolOutput)
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
                    MechaCodeSymbolThinking::new(
                        code_symbol,
                        ordered_symbol.steps().to_owned(),
                        true,
                        ordered_symbol.file_path().to_owned(),
                        None,
                        vec![],
                    ),
                );
            } else {
                symbols_to_visit.insert(code_symbol.to_owned());
                final_code_snippets.insert(
                    code_symbol.to_owned(),
                    MechaCodeSymbolThinking::new(
                        code_symbol,
                        ordered_symbol.steps().to_owned(),
                        false,
                        ordered_symbol.file_path().to_owned(),
                        None,
                        vec![],
                    ),
                );
            }
        });
        symbols.iter().for_each(|symbol| {
            // if we do not have the new symbols being tracked here, we use it
            // for exploration
            if !new_symbols.contains(symbol.code_symbol()) {
                symbols_to_visit.insert(symbol.code_symbol().to_owned());
                if let Some(code_snippet) = final_code_snippets.get_mut(symbol.code_symbol()) {
                    code_snippet.add_step(symbol.thinking());
                }
            }
        });

        let mut mecha_symbols = vec![];

        for (_, mut code_snippet) in final_code_snippets.into_iter() {
            // we always open the document before asking for an outline
            let file_open_result = self
                .file_open(code_snippet.fs_file_path().to_owned())
                .await?;
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
                .get_symbols_outline(code_snippet.fs_file_path())
                .await;

            // We will either get an outline node or we will get None
            // for today, we will go with the following assumption
            // - if the document has already been open, then its good
            // - otherwise we open the document and parse it again
            if let Some(outline_nodes) = outline_nodes {
                let mut outline_nodes =
                    self.grab_symbols_from_outline(outline_nodes, code_snippet.symbol_name());

                // if there are no outline nodes, then we have to skip this part
                // and keep going
                if outline_nodes.is_empty() {
                    // here we need to do go-to-definition
                    // first we check where the symbol is present on the file
                    // and we can use goto-definition
                    // so we first search the file for where the symbol is
                    // this will be another invocation to the tools
                    // and then we ask for the definition once we find it
                    let file_data = self
                        .file_open(code_snippet.fs_file_path().to_owned())
                        .await?;
                    let file_content = file_data.contents();
                    // now we parse it and grab the outline nodes
                    let find_in_file = self
                        .find_in_file(file_content, code_snippet.symbol_name().to_owned())
                        .await
                        .map(|find_in_file| find_in_file.get_position())
                        .ok()
                        .flatten();
                    // now that we have a poition, we can ask for go-to-definition
                    if let Some(file_position) = find_in_file {
                        let definition = self
                            .go_to_definition(&code_snippet.fs_file_path(), file_position)
                            .await?;
                        // let definition_file_path = definition.file_path().to_owned();
                        let snippet_node = self
                            .grab_symbol_content_from_definition(
                                &code_snippet.symbol_name(),
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
                        outline_node.content().to_owned(),
                    ));
                }
            } else {
                // if this is new, then we probably do not have a file path
                // to write it to
                if !code_snippet.is_new() {
                    // its a symbol but we have nothing about it, so we log
                    // this as error for now, but later we have to figure out
                    // what to do about it
                    println!(
                        "this is pretty bad, read the comment above on what is happening {:?}",
                        &code_snippet.symbol_name()
                    );
                }
            }

            mecha_symbols.push(code_snippet);
        }
        Ok(mecha_symbols)
    }

    pub async fn go_to_implementation(
        &self,
        snippet: &Snippet,
        symbol_name: &str,
    ) -> Result<GoToImplementationResponse, SymbolError> {
        // LSP requies the EXACT symbol location on where to click go-to-implementation
        // since thats the case we can just open the file and then look for the
        // first occurance of the symbol and grab the location
        let file_content = self.file_open(snippet.file_path().to_owned()).await?;
        let find_in_file = self
            .find_in_file(file_content.contents(), symbol_name.to_owned())
            .await?;
        if let Some(position) = find_in_file.get_position() {
            let request = ToolInput::SymbolImplementations(GoToImplementationRequest::new(
                snippet.file_path().to_owned(),
                position,
                self.editor_url.to_owned(),
            ));
            self.ui_events.send(UIEvent::from(request.clone()));
            self.tools
                .invoke(request)
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
                        symbol.content().to_owned(),
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
}
