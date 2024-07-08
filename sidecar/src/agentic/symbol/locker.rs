//! Contains a lock on the different symbols and maintains them running in memory
//! this way we are able to manage different symbols and their run-time while running
//! them in a session.
//! Symbol locker has access to the whole fs file-system and can run searches
//! if the file path is not correct or incorrect, cause we have so much information
//! over here, if the symbol is properly defined we are sure to find it, even if there
//! are multiples we have enough context here to gather the information required
//! to create the correct symbol and send it over

use std::{collections::HashMap, sync::Arc};

use futures::lock::Mutex;
use tokio::sync::mpsc::UnboundedSender;

use crate::user_context::types::UserContext;

use super::{
    errors::SymbolError,
    events::types::SymbolEvent,
    identifier::{LLMProperties, MechaCodeSymbolThinking, SymbolIdentifier},
    tool_box::ToolBox,
    tool_properties::ToolProperties,
    types::{Symbol, SymbolEventRequest, SymbolEventResponse},
    ui_event::UIEventWithID,
};

#[derive(Clone)]
pub struct SymbolLocker {
    symbols: Arc<
        Mutex<
            HashMap<
                // TODO(skcd): what should be the key here for this to work properly
                // cause we can have multiple symbols which share the same name
                // this probably would not happen today but would be good to figure
                // out at some point
                // we need a human agent over here somehow, but where does it go?
                // do we make it a symbol itself or keep it somewhere else
                SymbolIdentifier,
                // this is the channel which we use to talk to this particular symbol
                // and everything related to it
                UnboundedSender<(
                    SymbolEvent,
                    String,
                    tokio::sync::oneshot::Sender<SymbolEventResponse>,
                )>,
            >,
        >,
    >,
    // this is the main communication channel which we can use to send requests
    // to the right symbol
    hub_sender: UnboundedSender<(
        SymbolEventRequest,
        String,
        tokio::sync::oneshot::Sender<SymbolEventResponse>,
    )>,
    tools: Arc<ToolBox>,
    llm_properties: LLMProperties,
    user_context: UserContext,
    ui_sender: UnboundedSender<UIEventWithID>,
}

impl SymbolLocker {
    pub fn new(
        hub_sender: UnboundedSender<(
            SymbolEventRequest,
            String,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>,
        tools: Arc<ToolBox>,
        llm_properties: LLMProperties,
        user_context: UserContext,
        ui_sender: UnboundedSender<UIEventWithID>,
    ) -> Self {
        Self {
            symbols: Arc::new(Mutex::new(HashMap::new())),
            hub_sender,
            tools,
            llm_properties,
            user_context,
            ui_sender,
        }
    }

    pub async fn process_request(
        &self,
        request_event: (
            SymbolEventRequest,
            String,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        ),
    ) {
        let request = request_event.0;
        let tool_properties = request.get_tool_properties().clone();
        let tool_properties_ref = &tool_properties;
        let request_id = request_event.1;
        let sender = request_event.2;
        let symbol_identifier = request.symbol().clone();
        let does_exist = {
            if self.symbols.lock().await.get(&symbol_identifier).is_some() {
                // if symbol already exists then we can just forward it to the symbol
                true
            } else {
                // the symbol does not exist and we have to create it first and then send it over
                false
            }
        };

        println!("Symbol: {:?} is up? {}", &symbol_identifier, does_exist);

        if !does_exist {
            if let Some(fs_file_path) = symbol_identifier.fs_file_path() {
                // grab the snippet for this symbol
                let snippet = self
                    .tools
                    .find_snippet_for_symbol(
                        &fs_file_path,
                        symbol_identifier.symbol_name(),
                        &request_id,
                    )
                    .await;
                if let Ok(snippet) = snippet {
                    // the symbol does not exist so we have to make sure that we can send it over somehow
                    let mecha_code_symbol_thinking = MechaCodeSymbolThinking::new(
                        symbol_identifier.symbol_name().to_owned(),
                        vec![],
                        false,
                        symbol_identifier.fs_file_path().expect("to present"),
                        Some(snippet),
                        vec![],
                        self.user_context.clone(),
                        self.tools.clone(),
                    );
                    // we create the symbol over here, but what about the context, I want
                    // to pass it to the symbol over here
                    let _ = self
                        .create_symbol_agent(
                            mecha_code_symbol_thinking,
                            request_id.to_owned(),
                            tool_properties_ref.clone(),
                        )
                        .await;
                } else {
                    // we are fucked over here since we didn't find a snippet for the symbol
                    // which is supposed to have some presence in the file
                    println!(
                        "Snippet not found for: {} in {}",
                        symbol_identifier.symbol_name(),
                        fs_file_path
                    );
                    println!("no snippet found for the snippet, we are screwed over here, look at the comment above, for symbol");
                    return;
                    // todo!("no snippet found for the snippet, we are screwed over here, look at the comment above, for symbol");
                }
            } else {
                // well this kind of sucks, cause we do not know where the symbol is anymore
                // worst case this means that we have to create a new symbol somehow
                // best case this could mean that we fucked up majorly somewhere... what should we do???
                println!("we are mostly fucked if this is the case, we have to figure out how to handle the request coming in but not having the file path later on");
                return;
                // todo!("we are mostly fucked if this is the case, we have to figure out how to handle the request coming in but not having the file path later on")
            }
        }

        // at this point we have also tried creating the symbol agent, so we can start logging it
        {
            if let Some(symbol) = self.symbols.lock().await.get(&symbol_identifier) {
                let response = symbol.send((request.remove_event(), request_id, sender));
                println!("Request sending erorr: {:?}", response.is_err());
            }
        }
    }

    pub async fn create_symbol_agent(
        &self,
        request: MechaCodeSymbolThinking,
        request_id: String,
        tool_properties: ToolProperties,
    ) -> Result<SymbolIdentifier, SymbolError> {
        // say we create the symbol agent, what happens next
        // the agent can have its own events which it might need to do, including the
        // followups or anything else
        // the user might have some events to send
        // other agents might also want to talk to it for some information
        let symbol_identifier = request.to_symbol_identifier();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<(
            SymbolEvent,
            String,
            tokio::sync::oneshot::Sender<SymbolEventResponse>,
        )>();
        {
            println!("create_symbol_agent: {}", symbol_identifier.symbol_name());
            let mut symbols = self.symbols.lock().await;
            symbols.insert(symbol_identifier.clone(), sender);
            println!(
                "self.symbols.contains(&{}):({})",
                &symbol_identifier.symbol_name(),
                symbols.get(&symbol_identifier).is_some(),
            );
        }

        // now we create the symbol and let it rip
        let symbol_name = symbol_identifier.symbol_name().to_owned();
        let symbol = Symbol::new(
            symbol_identifier.clone(),
            request,
            self.hub_sender.clone(),
            self.tools.clone(),
            self.llm_properties.clone(),
            self.ui_sender.clone(),
            request_id,
            tool_properties,
        )
        .await;

        println!(
            "Symbol::new({:?}) is_err: {:?} symbol: {:?}",
            symbol_name,
            symbol.is_err(),
            &symbol,
        );

        let symbol = symbol?;

        let cloned_symbol_identifier = symbol_identifier.clone();

        // now we let it rip, we give the symbol the receiver and ask it
        // to go crazy with it
        let _symbol_run_handle = tokio::spawn(async move {
            println!("spawning symbol: {:?}.run()", &symbol_identifier);
            let response = symbol.run(receiver).await;
            println!("symbol stopped: {:?}.stop()", symbol_identifier);
            println!("{:?}", response.is_err());
        });
        // fin
        Ok(cloned_symbol_identifier)
    }
}
