use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::{stream, StreamExt};
use llm_client::clients::types::LLMType;
use llm_client::provider::{LLMProvider, LLMProviderAPIKeys};
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::symbol::helpers::split_file_content_into_parts;
use crate::agentic::symbol::identifier::{Snippet, SymbolIdentifier};
use crate::agentic::tool::base::Tool;
use crate::agentic::tool::code_edit::types::CodeEdit;
use crate::agentic::tool::code_symbol::correctness::{
    CodeCorrectnessAction, CodeCorrectnessRequest,
};
use crate::agentic::tool::code_symbol::important::{
    CodeSymbolImportantRequest, CodeSymbolImportantResponse, CodeSymbolImportantWideSearch,
    CodeSymbolUtilityRequest, CodeSymbolWithThinking,
};
use crate::agentic::tool::editor::apply::{EditorApplyRequest, EditorApplyResponse};
use crate::agentic::tool::errors::ToolError;
use crate::agentic::tool::filtering::broker::{
    CodeToEditFilterRequest, CodeToEditFilterResponse, CodeToEditSymbolRequest,
    CodeToEditSymbolResponse,
};
use crate::agentic::tool::grep::file::{FindInFileRequest, FindInFileResponse};
use crate::agentic::tool::lsp::diagnostics::{
    Diagnostic, LSPDiagnosticsInput, LSPDiagnosticsOutput,
};
use crate::agentic::tool::lsp::gotodefintion::{GoToDefinitionRequest, GoToDefinitionResponse};
use crate::agentic::tool::lsp::gotoimplementations::{
    GoToImplementationRequest, GoToImplementationResponse,
};
use crate::agentic::tool::lsp::open_file::OpenFileResponse;
use crate::agentic::tool::lsp::quick_fix::{
    GetQuickFixRequest, GetQuickFixResponse, LSPQuickFixInvocationRequest,
    LSPQuickFixInvocationResponse, QuickFixOption,
};
use crate::chunking::editor_parsing::EditorParsing;
use crate::chunking::text_document::{Position, Range};
use crate::chunking::types::{OutlineNode, OutlineNodeContent};
use crate::user_context::types::UserContext;
use crate::{
    agentic::tool::{broker::ToolBroker, input::ToolInput, lsp::open_file::OpenFileRequest},
    inline_completion::symbols_tracker::SymbolTrackerInline,
};

use super::errors::SymbolError;
use super::events::edit::SymbolToEdit;
use super::identifier::MechaCodeSymbolThinking;
use super::types::{SymbolEventRequest, SymbolEventResponse};
use super::ui_event::UIEvent;

#[derive(Clone)]
pub struct ToolBox {
    tools: Arc<ToolBroker>,
    symbol_broker: Arc<SymbolTrackerInline>,
    editor_parsing: Arc<EditorParsing>,
    editor_url: String,
    ui_events: UnboundedSender<UIEvent>,
}

impl ToolBox {
    pub fn new(
        tools: Arc<ToolBroker>,
        symbol_broker: Arc<SymbolTrackerInline>,
        editor_parsing: Arc<EditorParsing>,
        editor_url: String,
        ui_events: UnboundedSender<UIEvent>,
    ) -> Self {
        Self {
            tools,
            symbol_broker,
            editor_parsing,
            editor_url,
            ui_events,
        }
    }

    pub async fn find_symbol_to_edit(
        &self,
        symbol_to_edit: &SymbolToEdit,
    ) -> Result<OutlineNodeContent, SymbolError> {
        let outline_nodes = self
            .get_outline_nodes(symbol_to_edit.fs_file_path())
            .await
            .ok_or(SymbolError::ExpectedFileToExist)?;
        let mut filtered_outline_nodes = outline_nodes
            .into_iter()
            .filter(|outline_node| outline_node.name() == symbol_to_edit.symbol_name())
            .collect::<Vec<OutlineNodeContent>>();
        // There can be multiple nodes here which have the same name, we need to pick
        // the one we are interested in, an easy way to check this is to literally
        // check the absolute distance between the symbol we want to edit and the symbol
        filtered_outline_nodes.sort_by(|outline_node_first, outline_node_second| {
            // does it sort properly
            let distance_first: i64 = if symbol_to_edit
                .range()
                .intersects_without_byte(outline_node_first.range())
            {
                0
            } else {
                symbol_to_edit
                    .range()
                    .minimal_line_distance(outline_node_first.range())
            };

            let distance_second: i64 = if symbol_to_edit
                .range()
                .intersects_without_byte(outline_node_second.range())
            {
                0
            } else {
                symbol_to_edit
                    .range()
                    .minimal_line_distance(outline_node_second.range())
            };
            distance_first.cmp(&distance_second)
        });
        if filtered_outline_nodes.is_empty() {
            Err(SymbolError::SymbolNotFound)
        } else {
            Ok(filtered_outline_nodes.remove(0))
        }
    }

    pub fn detect_language(&self, fs_file_path: &str) -> Option<String> {
        self.editor_parsing
            .for_file_path(fs_file_path)
            .map(|ts_language_config| ts_language_config.language_str.to_owned())
    }

    pub async fn utlity_symbols_search(
        &self,
        user_query: &str,
        already_collected_definitions: &[&CodeSymbolWithThinking],
        outline_node_content: &OutlineNodeContent,
        fs_file_content: &str,
        fs_file_path: &str,
        user_context: &UserContext,
        language: &str,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
    ) -> Result<Vec<Option<(CodeSymbolWithThinking, String)>>, SymbolError> {
        // we are going to use the long context search here to check if there are
        // other utility functions we can and should use for implementing this feature
        // In our user-query we tell the LLM about what symbols are already included
        // and we ask the LLM to collect the other utility symbols which are missed

        // we have to create the query here using the outline node we are interested in
        // and the definitions which we already know about
        let request = CodeSymbolUtilityRequest::new(
            user_query.to_owned(),
            already_collected_definitions
                .into_iter()
                .map(|symbol_with_thinking| {
                    let file_path = symbol_with_thinking.file_path();
                    let symbol_name = symbol_with_thinking.code_symbol();
                    // TODO(skcd): This is horribly wrong, we want to get the full symbol
                    // over here and not just the symbol name since that does not make sense
                    // or at the very least the outline for the symbol
                    format!(
                        r#"<snippet>
<file_path>
{file_path}
</file_path>
<symbol_name>
{symbol_name}
</symbol_name>
</snippet>"#
                    )
                })
                .collect::<Vec<_>>(),
            fs_file_path.to_owned(),
            fs_file_content.to_owned(),
            outline_node_content.range().clone(),
            language.to_owned(),
            llm,
            provider,
            api_keys,
            user_context.clone(),
        );
        let tool_input = ToolInput::CodeSymbolUtilitySearch(request);
        let _ = self.ui_events.send(UIEvent::ToolEvent(tool_input.clone()));
        // These are the code symbols which are important from the global search
        // we might have some errors over here which we should fix later on, but we
        // will get on that
        // TODO(skcd): Figure out the best way to fix them
        // pick up from here, we need to run some cleanup things over here, to make sure
        // that we dont make mistakes while grabbing the code symbols
        // for now: we can assume that there are no errors over here, we can work with
        // this assumption for now
        let code_symbols = self
            .tools
            .invoke(tool_input)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .utility_code_search_response()
            .ok_or(SymbolError::WrongToolOutput)?;

        let file_paths_to_open: Vec<String> = code_symbols
            .symbols()
            .iter()
            .map(|symbol| symbol.file_path().to_owned())
            .collect::<Vec<_>>();
        // We have the file content for the file paths which the retrival
        // engine presented us with
        let file_to_content_mapping = stream::iter(file_paths_to_open)
            .map(|file_to_open| async move {
                let tool_input = ToolInput::OpenFile(OpenFileRequest::new(
                    file_to_open.to_owned(),
                    self.editor_url.to_owned(),
                ));
                (
                    file_to_open,
                    self.tools
                        .invoke(tool_input)
                        .await
                        .map(|tool_output| tool_output.get_file_open_response()),
                )
            })
            .buffer_unordered(100)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(
                |(fs_file_path, open_file_response)| match open_file_response {
                    Ok(Some(response)) => Some((fs_file_path, response.contents())),
                    _ => None,
                },
            )
            .collect::<HashMap<String, String>>();
        // After this we want to grab the symbol definition after looking at where
        // the symbol is in the file
        let symbols_to_grab = code_symbols.remove_symbols();
        let symbol_locations = stream::iter(symbols_to_grab)
            .map(|symbol| async {
                let symbol_name = symbol.code_symbol();
                let fs_file_path = symbol.file_path();
                if let Some(file_content) = file_to_content_mapping.get(fs_file_path) {
                    let location = self.find_symbol_in_file(symbol_name, file_content).await;
                    Some((symbol, location))
                } else {
                    None
                }
            })
            .buffer_unordered(100)
            .filter_map(|content| futures::future::ready(content))
            .collect::<Vec<_>>()
            .await;

        // We now have the locations and the symbol as well, we now ask the symbol manager
        // for the outline for this symbol
        let symbol_to_definition = stream::iter(
            symbol_locations
                .into_iter()
                .map(|symbol_location| (symbol_location, hub_sender.clone())),
        )
        .map(|((symbol, location), hub_sender)| async move {
            if let Ok(location) = location {
                // we might not get the position here for some weird reason which
                // is also fine
                let position = location.get_position();
                if let Some(position) = position {
                    let possible_file_path = self
                        .go_to_definition(fs_file_path, position)
                        .await
                        .map(|position| {
                            // there are multiple definitions here for some
                            // reason which I can't recall why, but we will
                            // always take the first one and run with it cause
                            // we then let this symbol agent take care of things
                            // TODO(skcd): The symbol needs to be on the
                            // correct file path over here
                            let symbol_file_path = position
                                .definitions()
                                .first()
                                .map(|definition| definition.file_path().to_owned());
                            symbol_file_path
                        })
                        .ok()
                        .flatten();
                    if let Some(definition_file_path) = possible_file_path {
                        let (sender, receiver) = tokio::sync::oneshot::channel();
                        // we have the possible file path over here
                        let _ = hub_sender.send((
                            SymbolEventRequest::outline(SymbolIdentifier::with_file_path(
                                symbol.code_symbol(),
                                &definition_file_path,
                            )),
                            sender,
                        ));
                        receiver
                            .await
                            .map(|response| response.to_string())
                            .ok()
                            .map(|definition_outline| (symbol, definition_outline))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;
        Ok(symbol_to_definition)
    }

    pub async fn check_code_correctness(
        &self,
        symbol_edited: &SymbolToEdit,
        original_code: &str,
        edited_code: &str,
        // this is the context from the code edit which we want to keep using while
        // fixing
        code_edit_extra_context: &str,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
    ) -> Result<(), SymbolError> {
        // code correction looks like this:
        // - apply the edited code to the original selection
        // - look at the code actions which are available
        // - take one of the actions or edit code as required
        // - once we have no LSP errors or anything we are good
        let instructions = symbol_edited.instructions().join("\n");
        let fs_file_path = symbol_edited.fs_file_path();
        let symbol_name = symbol_edited.symbol_name();
        let mut tries = 0;
        let max_tries = 5;
        loop {
            // keeping a try counter
            if tries >= max_tries {
                break;
            }
            tries = tries + 1;

            let symbol_to_edit = self.find_symbol_to_edit(symbol_edited).await?;
            let fs_file_content = self.file_open(fs_file_path.to_owned()).await?.contents();

            let updated_code = edited_code.to_owned();
            let edited_range = symbol_to_edit.range().clone();
            let request_id = uuid::Uuid::new_v4().to_string();
            let editor_response = self
                .apply_edits_to_editor(fs_file_path, &edited_range, &updated_code)
                .await?;

            // Now we check for LSP diagnostics
            let lsp_diagnostics = self
                .get_lsp_diagnostics(fs_file_path, &edited_range)
                .await?;

            // We also give it the option to edit the code as required
            if lsp_diagnostics.get_diagnostics().is_empty() {
                break;
            }

            // Now we get all the quick fixes which are available in the editor
            let quick_fix_actions = self
                .get_quick_fix_actions(fs_file_path, &edited_range, request_id.to_owned())
                .await?
                .remove_options();

            // now we can send over the request to the LLM to select the best tool
            // for editing the code out
            let selected_action = self
                .code_correctness_action_selection(
                    fs_file_path,
                    &fs_file_content,
                    &edited_range,
                    symbol_name,
                    &instructions,
                    original_code,
                    lsp_diagnostics.remove_diagnostics(),
                    quick_fix_actions.to_vec(),
                    llm.clone(),
                    provider.clone(),
                    api_keys.clone(),
                )
                .await?;

            // Now that we have the selected action, we can chose what to do about it
            // there might be a case that we have to re-write the code completely, since
            // the LLM thinks that the best thing to do, or invoke one of the quick-fix actions
            let selected_action_index = selected_action.index();
            let selected_action_thinking = selected_action.thinking();

            // code edit is a special operation which is not present in the quick-fix
            // but is provided by us, the way to check this is by looking at the index and seeing
            // if its >= length of the quick_fix_actions (we append to it internally in the LLM call)
            if selected_action_index >= quick_fix_actions.len() as i64 {
                todo!("implement the code edit flow over here")
            } else {
                // invoke the code action over here with the ap
                let response = self
                    .invoke_quick_action(selected_action_index, &request_id)
                    .await?;
                if response.is_success() {
                    // great we have a W
                } else {
                    // boo something bad happened, we should probably log and do something about this here
                    // for now we assume its all Ws
                }
            }
        }
        // todo!("we have to figure out this loop properly");
        todo!("we want to complete this")
    }

    async fn code_correctness_action_selection(
        &self,
        fs_file_path: &str,
        fs_file_content: &str,
        edited_range: &Range,
        symbol_name: &str,
        instruction: &str,
        previous_code: &str,
        diagnostics: Vec<Diagnostic>,
        quick_fix_actions: Vec<QuickFixOption>,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
    ) -> Result<CodeCorrectnessAction, SymbolError> {
        let (code_above, code_below, code_in_selection) =
            split_file_content_into_parts(fs_file_content, edited_range);
        let request = ToolInput::CodeCorrectnessAction(CodeCorrectnessRequest::new(
            fs_file_content.to_owned(),
            fs_file_path.to_owned(),
            code_above,
            code_below,
            code_in_selection,
            symbol_name.to_owned(),
            instruction.to_owned(),
            diagnostics,
            quick_fix_actions,
            previous_code.to_owned(),
            llm,
            provider,
            api_keys,
        ));
        let _ = self.ui_events.send(UIEvent::ToolEvent(request.clone()));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_code_correctness_action()
            .ok_or(SymbolError::WrongToolOutput)
    }

    pub async fn code_edit(
        &self,
        fs_file_path: &str,
        file_content: &str,
        selection_range: &Range,
        extra_context: String,
        instruction: String,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
    ) -> Result<String, SymbolError> {
        // we need to get the range above and then below and then in the selection
        let language = self
            .editor_parsing
            .for_file_path(fs_file_path)
            .map(|language_config| language_config.get_language())
            .flatten()
            .unwrap_or("".to_owned());
        let (above, below, in_range_selection) =
            split_file_content_into_parts(file_content, selection_range);
        let request = ToolInput::CodeEditing(CodeEdit::new(
            above,
            below,
            fs_file_path.to_owned(),
            in_range_selection,
            extra_context,
            language.to_owned(),
            instruction,
            llm,
            api_keys,
            provider,
        ));
        let _ = self.ui_events.send(UIEvent::ToolEvent(request.clone()));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_code_edit_output()
            .ok_or(SymbolError::WrongToolOutput)
    }

    async fn invoke_quick_action(
        &self,
        quick_fix_index: i64,
        request_id: &str,
    ) -> Result<LSPQuickFixInvocationResponse, SymbolError> {
        let request = ToolInput::QuickFixInvocationRequest(LSPQuickFixInvocationRequest::new(
            request_id.to_owned(),
            quick_fix_index,
            self.editor_url.to_owned(),
        ));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_quick_fix_invocation_result()
            .ok_or(SymbolError::WrongToolOutput)
    }

    pub async fn get_file_content(&self, fs_file_path: &str) -> Result<String, SymbolError> {
        self.symbol_broker
            .get_file_content(fs_file_path)
            .await
            .ok_or(SymbolError::UnableToReadFileContent)
    }

    pub async fn gather_important_symbols_with_definition(
        &self,
        fs_file_path: &str,
        file_content: &str,
        selection_range: &Range,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        query: &str,
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
        // we get back here the defintion outline along with the reasoning on why
        // we need to look at the symbol
    ) -> Result<Vec<Option<(CodeSymbolWithThinking, String)>>, SymbolError> {
        let language = self
            .editor_parsing
            .for_file_path(fs_file_path)
            .map(|language_config| language_config.get_language())
            .flatten()
            .unwrap_or("".to_owned());
        let request = ToolInput::RequestImportantSymbols(CodeSymbolImportantRequest::new(
            None,
            vec![],
            fs_file_path.to_owned(),
            file_content.to_owned(),
            selection_range.clone(),
            llm,
            provider,
            api_keys,
            // TODO(skcd): fill in the language over here by using editor parsing
            language,
            query.to_owned(),
        ));
        let _ = self.ui_events.send(UIEvent::from(request.clone()));
        let response = self
            .tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_important_symbols()
            .ok_or(SymbolError::WrongToolOutput)?;
        let symbols_to_grab = response
            .symbols()
            .into_iter()
            .map(|symbol| symbol.clone())
            .collect::<Vec<_>>();
        let symbol_locations = stream::iter(symbols_to_grab)
            .map(|symbol| async move {
                let symbol_name = symbol.code_symbol();
                let location = self.find_symbol_in_file(symbol_name, file_content).await;
                (symbol, location)
            })
            .buffer_unordered(100)
            .collect::<Vec<_>>()
            .await;

        // we want to grab the defintion of these symbols over here, so we can either
        // ask the hub and get it back or do something else... asking the hub is the best
        // thing to do over here
        // we now need to go to the definitions of these symbols and then ask the hub
        // manager to grab the outlines
        let symbol_to_definition = stream::iter(
            symbol_locations
                .into_iter()
                .map(|symbol_location| (symbol_location, hub_sender.clone())),
        )
        .map(|((symbol, location), hub_sender)| async move {
            if let Ok(location) = location {
                // we might not get the position here for some weird reason which
                // is also fine
                let position = location.get_position();
                if let Some(position) = position {
                    let possible_file_path = self
                        .go_to_definition(fs_file_path, position)
                        .await
                        .map(|position| {
                            // there are multiple definitions here for some
                            // reason which I can't recall why, but we will
                            // always take the first one and run with it cause
                            // we then let this symbol agent take care of things
                            // TODO(skcd): The symbol needs to be on the
                            // correct file path over here
                            let symbol_file_path = position
                                .definitions()
                                .first()
                                .map(|definition| definition.file_path().to_owned());
                            symbol_file_path
                        })
                        .ok()
                        .flatten();
                    if let Some(definition_file_path) = possible_file_path {
                        let (sender, receiver) = tokio::sync::oneshot::channel();
                        // we have the possible file path over here
                        let _ = hub_sender.send((
                            SymbolEventRequest::outline(SymbolIdentifier::with_file_path(
                                symbol.code_symbol(),
                                &definition_file_path,
                            )),
                            sender,
                        ));
                        receiver
                            .await
                            .map(|response| response.to_string())
                            .ok()
                            .map(|definition_outline| (symbol, definition_outline))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .buffer_unordered(100)
        .collect::<Vec<_>>()
        .await;
        Ok(symbol_to_definition)
    }

    async fn get_quick_fix_actions(
        &self,
        fs_file_path: &str,
        range: &Range,
        request_id: String,
    ) -> Result<GetQuickFixResponse, SymbolError> {
        let request = ToolInput::QuickFixRequest(GetQuickFixRequest::new(
            fs_file_path.to_owned(),
            self.editor_url.to_owned(),
            range.clone(),
            request_id,
        ));
        let _ = self.ui_events.send(UIEvent::ToolEvent(request.clone()));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_quick_fix_actions()
            .ok_or(SymbolError::WrongToolOutput)
    }

    async fn get_lsp_diagnostics(
        &self,
        fs_file_path: &str,
        range: &Range,
    ) -> Result<LSPDiagnosticsOutput, SymbolError> {
        let input = ToolInput::LSPDiagnostics(LSPDiagnosticsInput::new(
            fs_file_path.to_owned(),
            range.clone(),
            self.editor_url.to_owned(),
        ));
        let _ = self.ui_events.send(UIEvent::ToolEvent(input.clone()));
        self.tools
            .invoke(input)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_lsp_diagnostics()
            .ok_or(SymbolError::WrongToolOutput)
    }

    async fn apply_edits_to_editor(
        &self,
        fs_file_path: &str,
        range: &Range,
        updated_code: &str,
    ) -> Result<EditorApplyResponse, SymbolError> {
        let input = ToolInput::EditorApplyChange(EditorApplyRequest::new(
            fs_file_path.to_owned(),
            updated_code.to_owned(),
            range.clone(),
            self.editor_url.to_owned(),
        ));
        let _ = self.ui_events.send(UIEvent::ToolEvent(input.clone()));
        self.tools
            .invoke(input)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .get_editor_apply_response()
            .ok_or(SymbolError::WrongToolOutput)
    }

    async fn find_symbol_in_file(
        &self,
        symbol_name: &str,
        file_contents: &str,
    ) -> Result<FindInFileResponse, SymbolError> {
        // Here we are going to get the position of the symbol
        let request = ToolInput::GrepSingleFile(FindInFileRequest::new(
            file_contents.to_owned(),
            symbol_name.to_owned(),
        ));
        let _ = self.ui_events.send(UIEvent::from(request.clone()));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .grep_single_file()
            .ok_or(SymbolError::WrongToolOutput)
    }

    pub async fn filter_code_snippets_in_symbol_for_editing(
        &self,
        xml_string: String,
        query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
    ) -> Result<CodeToEditSymbolResponse, SymbolError> {
        let request = ToolInput::FilterCodeSnippetsForEditingSingleSymbols(
            CodeToEditSymbolRequest::new(xml_string, query, llm, provider, api_keys),
        );
        let _ = self.ui_events.send(UIEvent::from(request.clone()));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .code_to_edit_in_symbol()
            .ok_or(SymbolError::WrongToolOutput)
    }

    pub async fn get_outline_nodes(&self, fs_file_path: &str) -> Option<Vec<OutlineNodeContent>> {
        self.symbol_broker
            .get_symbols_outline(&fs_file_path)
            .await
            .map(|outline_nodes| {
                // class and the functions are included here
                outline_nodes
                    .into_iter()
                    .map(|outline_node| {
                        // let children = outline_node.consume_all_outlines();
                        // outline node here contains the classes and the functions
                        // which we have to edit
                        // so one way would be to ask the LLM to edit it
                        // another is to figure out if we can show it all the functions
                        // which are present inside the class and ask it to make changes
                        let outline_content = outline_node.content().clone();
                        let all_outlines = outline_node.consume_all_outlines();
                        vec![outline_content]
                            .into_iter()
                            .chain(all_outlines)
                            .collect::<Vec<OutlineNodeContent>>()
                    })
                    .flatten()
                    .collect::<Vec<_>>()
            })
    }

    pub async fn symbol_in_range(
        &self,
        fs_file_path: &str,
        range: &Range,
    ) -> Option<Vec<OutlineNode>> {
        self.symbol_broker
            .get_symbols_in_range(fs_file_path, range)
            .await
    }

    // TODO(skcd): Use this to ask the LLM for the code snippets which need editing
    pub async fn filter_code_for_editing(
        &self,
        snippets: Vec<Snippet>,
        query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_key: LLMProviderAPIKeys,
    ) -> Result<CodeToEditFilterResponse, SymbolError> {
        let request = ToolInput::FilterCodeSnippetsForEditing(CodeToEditFilterRequest::new(
            snippets, query, llm, provider, api_key,
        ));
        let _ = self.ui_events.send(UIEvent::from(request.clone()));
        self.tools
            .invoke(request)
            .await
            .map_err(|e| SymbolError::ToolError(e))?
            .code_to_edit_filter()
            .ok_or(SymbolError::WrongToolOutput)
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
        user_context: UserContext,
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
                        user_context.clone(),
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
                        user_context.clone(),
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
                        outline_node,
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
                .into_iter()
                .find(|symbol| symbol.name() == symbol_name)
                .map(|symbol| {
                    Snippet::new(
                        symbol.name().to_owned(),
                        symbol.range().clone(),
                        definition.file_path().to_owned(),
                        symbol.content().to_owned(),
                        symbol,
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
