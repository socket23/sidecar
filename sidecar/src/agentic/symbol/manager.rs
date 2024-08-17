//! Contains the central manager for the symbols and maintains them in the system
//! as a connected graph in some ways in which these symbols are able to communicate
//! with each other

use std::sync::Arc;

use futures::{stream, StreamExt};
use llm_client::clients::types::LLMType;
use llm_client::provider::{GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys, OpenAIProvider};
use tokio::sync::mpsc::UnboundedSender;

use crate::agentic::swe_bench::search_cache::LongContextSearchCache;
use crate::agentic::symbol::events::edit::SymbolToEdit;
use crate::agentic::symbol::events::initial_request::{InitialRequestData, SymbolEditedItem};
use crate::agentic::symbol::events::probe::SymbolToProbeRequest;
use crate::agentic::symbol::events::types::SymbolEvent;
use crate::agentic::symbol::tool_properties::ToolProperties;
use crate::agentic::symbol::ui_event::InitialSearchSymbolInformation;
use crate::agentic::tool::code_symbol::important::CodeSymbolImportantWideSearch;
use crate::agentic::tool::input::ToolInput;
use crate::agentic::tool::r#type::Tool;
use crate::chunking::editor_parsing::EditorParsing;
use crate::chunking::languages::TSLanguageParsing;
use crate::chunking::text_document::{Position, Range};
use crate::user_context::types::UserContext;
use crate::{
    agentic::tool::{broker::ToolBroker, output::ToolOutput},
    inline_completion::symbols_tracker::SymbolTrackerInline,
};

use super::identifier::LLMProperties;
use super::tool_box::ToolBox;
use super::ui_event::UIEventWithID;
use super::{
    errors::SymbolError,
    events::input::SymbolInputEvent,
    locker::SymbolLocker,
    types::{SymbolEventRequest, SymbolEventResponse},
};

// This is the main communication manager between all the symbols
// this of this as the central hub through which all the events go forward
pub struct SymbolManager {
    _sender: UnboundedSender<(
        SymbolEventRequest,
        String,
        // This is the ui sender which we send along, because each request the
        // agent is working on can be part of a different query from the server
        tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
        tokio::sync::oneshot::Sender<SymbolEventResponse>,
    )>,
    // this is the channel where the various symbols will use to talk to the manager
    // which in turn will proxy it to the right symbol, what happens if there are failures
    // each symbol has its own receiver which is being used
    symbol_locker: SymbolLocker,
    tools: Arc<ToolBroker>,
    _symbol_broker: Arc<SymbolTrackerInline>,
    _editor_parsing: Arc<EditorParsing>,
    ts_parsing: Arc<TSLanguageParsing>,
    tool_box: Arc<ToolBox>,
    _editor_url: String,
    llm_properties: LLMProperties,
    long_context_cache: LongContextSearchCache,
    root_request_id: String,
}

impl SymbolManager {
    pub fn new(
        tools: Arc<ToolBroker>,
        symbol_broker: Arc<SymbolTrackerInline>,
        editor_parsing: Arc<EditorParsing>,
        editor_url: String,
        llm_properties: LLMProperties,
        // This is a hack and not a proper one at that, we obviously want to
        // do better over here
        user_context: UserContext,
        request_id: String,
    ) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<(
            SymbolEventRequest,
            String,
            tokio::sync::mpsc::UnboundedSender<UIEventWithID>,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>();
        let tool_box = Arc::new(ToolBox::new(
            tools.clone(),
            symbol_broker.clone(),
            editor_parsing.clone(),
            editor_url.to_owned(),
            request_id.to_owned(),
        ));
        let symbol_locker = SymbolLocker::new(
            sender.clone(),
            tool_box.clone(),
            llm_properties.clone(),
            user_context,
        );
        let cloned_symbol_locker = symbol_locker.clone();
        tokio::spawn(async move {
            // TODO(skcd): Make this run in full parallelism in the future, for
            // now this is fine
            while let Some(event) = receiver.recv().await {
                println!("symbol_manager::tokio::spawn::receiver_event");
                // let _ = cloned_ui_sender.send(UIEvent::from(event.0.clone()));
                let _ = cloned_symbol_locker.process_request(event).await;
            }
            println!("symbol_manager::tokio::spawn::end");
        });
        let ts_parsing = Arc::new(TSLanguageParsing::init());
        Self {
            _sender: sender,
            symbol_locker,
            _editor_parsing: editor_parsing,
            ts_parsing,
            tools,
            _symbol_broker: symbol_broker,
            tool_box,
            _editor_url: editor_url,
            llm_properties,
            long_context_cache: LongContextSearchCache::new(),
            root_request_id: request_id.to_owned(),
        }
    }

    pub async fn impls_test(&self, ui_sender: UnboundedSender<UIEventWithID>) {
        // simulating an edit has occured in range
        let path =
            "/Users/zi/codestory/testing/sidecar/sidecar/src/agentic/tool/search/iterative.rs";
        let start_position = Position::new(81, 0, 420);
        let end_position = Position::new(89, 0, 420);
        let edited_range = Range::new(start_position, end_position);

        let outline_node = self
            .tool_box
            .get_outline_node_for_range(&edited_range, &path, "request_id", ui_sender.clone())
            .await
            .unwrap();

        let node_name = outline_node.name();
        let original_code = outline_node.content().content();
        let _outline_node_range = outline_node.range();

        let identifier_range = outline_node.identifier_range();

        let symbol_to_edit = SymbolToEdit::new(
            node_name.to_string(),
            identifier_range.to_owned(), // symbol range is the the outline node's range (complete range)
            path.to_string(),
            vec!["some instruction, cook eggs".to_string()],
            false,
            false,
            false,
            "please cook eggs".to_string(),
            None,
        );

        let _ = self
            .tool_box
            .check_for_followups(
                node_name,
                &symbol_to_edit,
                original_code,
                LLMType::Gpt4O,
                LLMProvider::OpenAI,
                LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
                    "sk-proj-BLaSMsWvoO6FyNwo9syqT3BlbkFJo3yqCyKAxWXLm4AvePtt".to_owned(),
                )),
                self.symbol_locker.hub_sender.clone(),
                ui_sender.clone(),
                &self.root_request_id,
                &ToolProperties::new(),
            )
            .await;

        // println!("{path}");
        // println!("start position: \n{:?}", &identifier_range.start_position());

        // let references = self
        //     .tool_box
        //     .go_to_references(
        //         path,
        //         &identifier_range.start_position(),
        //         &self.root_request_id,
        //     )
        //     .await
        //     .unwrap();

        // println!("{:?}", references);
    }

    pub fn tool_box(&self) -> &ToolBox {
        &self.tool_box
    }

    pub async fn probe_request_from_user_context(
        &self,
        query: String,
        user_context: UserContext,
        ui_sender: UnboundedSender<UIEventWithID>,
    ) -> Result<(), SymbolError> {
        println!("symbol_manager::probe_request_from_user_context::start");
        let request_id = uuid::Uuid::new_v4().to_string();
        let request_id_ref = &request_id;
        let variables = self
            .tool_box
            .grab_symbols_from_user_context(
                query.to_owned(),
                user_context.clone(),
                request_id.to_owned(),
                ui_sender.clone(),
            )
            .await;
        let outline = self
            .tool_box
            .outline_for_user_context(&user_context, &request_id, ui_sender.clone())
            .await;
        let code_wide_search =
            ToolInput::RequestImportantSymbolsCodeWide(CodeSymbolImportantWideSearch::new(
                user_context.clone(),
                query.to_owned(),
                // Hardcoding here, but we can remove this later
                LLMType::GeminiPro,
                LLMProvider::GoogleAIStudio,
                llm_client::provider::LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                    "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
                )),
                self.root_request_id.to_owned(),
                outline,
            ));
        let output = {
            match variables {
                Ok(variables) => ToolOutput::ImportantSymbols(variables),
                _ => self
                    .tools
                    .invoke(code_wide_search)
                    .await
                    .map_err(|e| SymbolError::ToolError(e))?,
            }
        };
        println!(
            "symbol_manager::probe_request_from_user_context::output({:?})",
            &output
        );
        if let ToolOutput::ImportantSymbols(important_symbols)
        | ToolOutput::RepoMapSearch(important_symbols) = output
        {
            // We have the important symbols here which we can then use to invoke the individual process request
            let important_symbols = important_symbols.fix_symbol_names(self.ts_parsing.clone());

            let mut symbols = self
                .tool_box
                .important_symbols(
                    &important_symbols,
                    user_context.clone(),
                    &request_id,
                    ui_sender.clone(),
                )
                .await
                .map_err(|e| e.into())?;
            // TODO(skcd): Another check over here is that we can search for the exact variable
            // and then ask for the edit
            println!(
                "symbol_manager::probe_request_from_user_context::[{}]",
                symbols
                    .iter()
                    .map(|(symbol, _)| symbol.symbol_name().to_owned())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            // TODO(skcd): the symbol here might belong to a class or it might be a global function
            // we want to grab the largest node containing the symbol here instead of using
            // the symbol directly since our algorithm would not work otherwise
            // we would also need to de-duplicate the symbols which we have to process right over here
            // otherwise it might lead to errors
            if symbols.is_empty() {
                println!("symbol_manager::grab_symbols_using_search");
                symbols = self
                    .tool_box
                    .grab_symbol_using_search(
                        important_symbols,
                        user_context.clone(),
                        &request_id,
                        ui_sender.clone(),
                    )
                    .await
                    .map_err(|e| e.into())?;
            }

            // Create all the symbol agents
            let symbol_identifiers = stream::iter(
                symbols
                    .into_iter()
                    .map(|symbol| (symbol, ui_sender.clone())),
            )
            .map(|((symbol_request, _), ui_sender)| async move {
                let symbol_identifier = self
                    .symbol_locker
                    .create_symbol_agent(
                        symbol_request,
                        request_id_ref.to_owned(),
                        ToolProperties::new(),
                        ui_sender,
                    )
                    .await;
                symbol_identifier
            })
            .buffer_unordered(100)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(|s| s.ok())
            .collect::<Vec<_>>();

            println!(
                "symbol_manager::probe_request_from_user_context::len({})",
                symbol_identifiers.len(),
            );

            let query_ref = &query;

            // Now for all the symbol identifiers which we are getting we have to
            // send a request to all of them with the probe query
            let responses = stream::iter(symbol_identifiers.into_iter().map(|symbol_identifier| {
                (
                    symbol_identifier.clone(),
                    request_id_ref.to_owned(),
                    SymbolEventRequest::probe_request(
                        symbol_identifier.clone(),
                        SymbolToProbeRequest::new(
                            symbol_identifier,
                            query_ref.to_owned(),
                            query_ref.to_owned(),
                            request_id_ref.to_owned(),
                            vec![],
                        ),
                        ToolProperties::new(),
                    ),
                    ui_sender.clone(),
                )
            }))
            .map(
                |(symbol_identifier, request_id, symbol_event_request, ui_sender)| async move {
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    dbg!(
                        "sending initial request to symbol: {:?}",
                        &symbol_identifier
                    );
                    self.symbol_locker
                        .process_request((
                            symbol_event_request,
                            request_id,
                            ui_sender.clone(),
                            sender,
                        ))
                        .await;
                    let response = receiver.await;
                    dbg!(
                        "For symbol identifier: {:?} the response is {:?}",
                        &symbol_identifier,
                        &response
                    );
                    (response, symbol_identifier)
                },
            )
            .buffer_unordered(100)
            .collect::<Vec<_>>()
            .await;

            // send the response forward after combining all the answers using the LLM
            let final_responses = responses
                .into_iter()
                .filter_map(|(response, symbol_identifier)| match response {
                    Ok(response) => Some((response, symbol_identifier)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let final_answer = final_responses
                .into_iter()
                .map(|(response, symbol_identifier)| {
                    let symbol_name = symbol_identifier.symbol_name();
                    let symbol_file_path = symbol_identifier
                        .fs_file_path()
                        .map(|fs_file_path| format!("at {}", fs_file_path))
                        .unwrap_or("".to_owned());
                    let response = response.to_string();
                    let symbol_readable = format!("{symbol_name} {symbol_file_path}");
                    format!(
                        r#"{symbol_readable}
{response}"#
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let _ = ui_sender.send(UIEventWithID::probing_finished_event(
                request_id_ref.to_owned(),
                final_answer,
            ));
            println!("things are completed over here after probing");
        }
        println!("things are more complete over here after probing");
        Ok(())
    }

    // This is just for testing out the flow for single input events
    pub async fn probe_request(
        &self,
        input_event: SymbolEventRequest,
        ui_sender: UnboundedSender<UIEventWithID>,
    ) -> Result<(), SymbolError> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request_id = uuid::Uuid::new_v4().to_string();
        let _ = self
            .symbol_locker
            .process_request((input_event, request_id, ui_sender.clone(), sender))
            .await;
        let response = receiver.await;
        println!("{:?}", response.expect("to work"));
        Ok(())
    }

    // once we have the initial request, which we will go through the initial request
    // mode once, we have the symbols from it we can use them to spin up sub-symbols as well
    pub async fn initial_request(&self, input_event: SymbolInputEvent) -> Result<(), SymbolError> {
        let user_context = input_event.provided_context().clone();
        let ui_sender = input_event.ui_sender();
        let request_id = input_event.request_id().to_owned();
        let is_full_edit = input_event.full_symbol_edit();
        let is_big_search = input_event.big_search();
        let swe_bench_id = input_event.swe_bench_instance_id();
        let swe_bench_git_dname = input_event.get_swe_bench_git_dname();
        let swe_bench_test_endpoint = input_event.get_swe_bench_test_endpoint();
        let swe_bench_code_editing_model = input_event.get_swe_bench_code_editing();
        let swe_bench_gemini_properties = input_event.get_swe_bench_gemini_llm_properties();
        let swe_bench_long_context_editing = input_event.get_swe_bench_long_context_editing();
        let full_symbol_edit = input_event.full_symbol_edit();
        let fast_code_symbol_llm = input_event.get_fast_code_symbol_llm();
        let tool_properties = ToolProperties::new()
            .set_swe_bench_endpoint(swe_bench_test_endpoint.clone())
            .set_swe_bench_code_editing_llm(swe_bench_code_editing_model)
            .set_swe_bench_reranking_llm(swe_bench_gemini_properties)
            .set_long_context_editing_llm(swe_bench_long_context_editing)
            .set_full_symbol_request(full_symbol_edit)
            .set_fast_code_symbol_search(fast_code_symbol_llm);
        let user_query = input_event.user_query().to_owned();
        let tool_input = input_event
            .tool_use_on_initial_invocation(self.tool_box.clone(), &request_id)
            .await;
        if let Some(tool_input) = tool_input {
            // send the tool input to the ui sender
            // we need some kind of cache here so we do not invoke the expensive
            // query so many times
            let important_symbols = if let Some(swe_bench_id) = swe_bench_id.to_owned() {
                let symbols = self.long_context_cache.check_cache(&swe_bench_id).await;
                if let Some(_git_dname) = swe_bench_git_dname {
                    match symbols {
                        Some(symbols) => Some(symbols),
                        None => None,
                    }
                } else {
                    symbols
                }
            } else {
                if request_id == "testing_code_editing_flow" {
                    self.long_context_cache.check_cache(&request_id).await
                } else {
                    None
                }
            };

            let tool_output = match important_symbols {
                Some(important_symbols) => ToolOutput::RepoMapSearch(important_symbols),
                None => {
                    if tool_input.is_repo_map_search() {
                        let _ = ui_sender.send(UIEventWithID::start_long_context_search(
                            request_id.to_owned(),
                        ));
                        let result = self
                            .tools
                            .invoke(tool_input)
                            .await
                            .map_err(|e| SymbolError::ToolError(e));
                        let _ = ui_sender.send(UIEventWithID::finish_long_context_search(
                            request_id.to_owned(),
                        ));
                        result?
                    } else {
                        self.tools
                            .invoke(tool_input)
                            .await
                            .map_err(|e| SymbolError::ToolError(e))?
                    }
                }
            };

            if let ToolOutput::ImportantSymbols(important_symbols)
            | ToolOutput::RepoMapSearch(important_symbols)
            | ToolOutput::BigSearch(important_symbols) = tool_output
            {
                // The fix symbol name here helps us get the top-level symbol name
                // if the LLM decides to have fun and spit out a.b.c instead of a or b or c individually
                // as it can with python where it will tell class.method_name instead of just class or just
                // method_name

                // should pass self.editorparsing <tsconfigs>
                let important_symbols = important_symbols.fix_symbol_names(self.ts_parsing.clone());
                // swe bench caching hit over here we just do it
                self.long_context_cache
                    .update_cache(swe_bench_id.clone(), &important_symbols)
                    .await;
                // TODO(codestory): Remove this after testing
                if request_id == "testing_code_editing_flow" {
                    let _ = self
                        .long_context_cache
                        .update_cache(Some(request_id.to_owned()), &important_symbols)
                        .await;
                }

                // Debug printing
                println!("Important symbols: {:?}", &important_symbols);

                println!("symbol_manager::planning_before_editing");
                let important_symbols = match self
                    .long_context_cache
                    .check_cache_for_plan_before_editing(
                        swe_bench_id
                            .clone()
                            .map(|_swe_bench_id| request_id.to_owned()),
                    )
                    .await
                {
                    Some(important_symbols) => important_symbols,
                    None => {
                        if is_full_edit && !is_big_search {
                            important_symbols
                        } else {
                            let important_symbols = self
                                .tool_box
                                .planning_before_code_editing(
                                    &important_symbols,
                                    &user_query,
                                    self.llm_properties.clone(),
                                    is_big_search,
                                    &request_id,
                                    ui_sender.clone(),
                                )
                                .await?
                                .fix_symbol_names(self.ts_parsing.clone());
                            self.long_context_cache
                                .update_cache_for_plan_before_editing(
                                    swe_bench_id.map(|_swe_bench_id| request_id.to_owned()),
                                    &important_symbols,
                                )
                                .await;
                            important_symbols
                        }
                    }
                };

                println!("symbol_manager::plan_finished_before_editing");

                let updated_tool_properties = important_symbols.ordered_symbols_to_plan();
                let tool_properties = tool_properties
                    .clone()
                    .set_plan_for_input(Some(updated_tool_properties));
                let tool_properties_ref = &tool_properties;
                println!(
                    "symbol_manager::tool_box::important_symbols::search({})",
                    important_symbols
                        .ordered_symbols()
                        .into_iter()
                        .map(|code_symbol| code_symbol.code_symbol())
                        .collect::<Vec<_>>()
                        .join(",")
                );

                // Lets first start another round of COT over here to figure out
                // how to go about making the changes, I know this is a bit orthodox
                // and goes against our plans of making the agents better, but
                // this feels useful to have, since the previous iteration
                // does not even see the code and what changes need to be made
                let mut symbols = self
                    .tool_box
                    .important_symbols(
                        &important_symbols,
                        user_context.clone(),
                        &request_id,
                        ui_sender.clone(),
                    )
                    .await
                    .map_err(|e| e.into())?;

                // send a UI event to the frontend over here
                let mut initial_symbol_search_information = vec![];
                for (symbol, _) in symbols.iter() {
                    initial_symbol_search_information.push(InitialSearchSymbolInformation::new(
                        symbol.symbol_name().to_owned(),
                        // TODO(codestory): umm.. how can we have a file path for a symbol
                        // which does not exist if is_new is true
                        Some(symbol.fs_file_path().to_owned()),
                        symbol.is_new(),
                        symbol.steps().await.join("\n"),
                        symbol
                            .get_snippet()
                            .await
                            .map(|snippet| snippet.range().clone()),
                    ));
                }
                let _ = ui_sender.send(UIEventWithID::initial_search_symbol_event(
                    request_id.to_owned(),
                    initial_symbol_search_information,
                ));
                // TODO(skcd): Another check over here is that we can search for the exact variable
                // and then ask for the edit
                println!(
                    "symbol_manager::initial_request::symbols::({})",
                    symbols
                        .iter()
                        .map(|(symbol, _)| symbol.symbol_name().to_owned())
                        .collect::<Vec<_>>()
                        .join(",")
                );

                let symbols_edited_list = important_symbols
                    .ordered_symbols()
                    .into_iter()
                    .map(|symbol| {
                        SymbolEditedItem::new(
                            symbol.code_symbol().to_owned(),
                            symbol.file_path().to_owned(),
                            symbol.is_new(),
                            symbol.steps().to_vec().join("\n"),
                        )
                    })
                    .collect::<Vec<_>>();
                // TODO(skcd): the symbol here might belong to a class or it might be a global function
                // we want to grab the largest node containing the symbol here instead of using
                // the symbol directly since our algorithm would not work otherwise
                // we would also need to de-duplicate the symbols which we have to process right over here
                // otherwise it might lead to errors
                if symbols.is_empty() {
                    println!("symbol_manager::grab_symbols_using_search");
                    symbols = self
                        .tool_box
                        .grab_symbol_using_search(
                            important_symbols,
                            user_context.clone(),
                            &request_id,
                            ui_sender.clone(),
                        )
                        .await
                        .map_err(|e| e.into())?;
                }

                println!("symbol_manager::symbols_len::({})", symbols.len());

                // This is where we are creating all the symbols
                let _ = stream::iter(
                    // we are loosing context about the changes which we want to make
                    // to the symbol over here
                    symbols.into_iter().map(|symbol| {
                        (
                            symbol,
                            request_id.to_owned(),
                            user_query.to_owned(),
                            symbols_edited_list.to_vec(),
                            ui_sender.clone(),
                        )
                    }),
                )
                .map(
                    |(
                        (symbol_request, steps),
                        request_id,
                        user_query,
                        symbols_edited_list,
                        ui_sender,
                    )| async move {
                        let symbol_name = symbol_request.symbol_name().to_owned();
                        let symbol_identifier = self
                            .symbol_locker
                            .create_symbol_agent(
                                symbol_request,
                                request_id.to_owned(),
                                tool_properties_ref.clone(),
                                ui_sender.clone(),
                            )
                            .await;
                        if let Ok(symbol_identifier) = symbol_identifier {
                            let symbol_event_request = SymbolEventRequest::new(
                                symbol_identifier.clone(),
                                SymbolEvent::InitialRequest(InitialRequestData::new(
                                    user_query.to_owned(),
                                    steps.join("\n"),
                                    // empty history when symbol manager sends the initial
                                    // request
                                    vec![],
                                    full_symbol_edit,
                                    Some(symbols_edited_list),
                                    is_big_search,
                                )),
                                tool_properties_ref.clone(),
                            );
                            let (sender, receiver) = tokio::sync::oneshot::channel();
                            println!(
                                "symbol_manager::initial_request::sending_request({})",
                                symbol_identifier.symbol_name()
                            );
                            self.symbol_locker
                                .process_request((
                                    symbol_event_request,
                                    request_id.to_owned(),
                                    ui_sender.clone(),
                                    sender,
                                ))
                                .await;
                            let response = receiver.await;
                            println!(
                                "symbol_manager::initial_request::response::({})::({:?})",
                                symbol_identifier.symbol_name(),
                                &response
                            );
                        } else {
                            println!(
                                "symbol_manager::initial_request::no_symbol_identifier({})",
                                symbol_name
                            );
                        }
                    },
                )
                .buffered(1)
                .collect::<Vec<_>>()
                .await;
            }
        } else {
            // We are for some reason not even invoking the first passage which is
            // weird, this is completely wrong and we should be replying back
            println!("this is wrong, please look at the comment here");
        }
        println!("symbol_manager::initial_request::finish");
        let _ = ui_sender.send(UIEventWithID::finish_edit_request(request_id));
        Ok(())
    }
}
