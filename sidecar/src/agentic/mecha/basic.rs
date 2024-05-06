//! This is the most basic of the mechas
//! The way this will work is the following:
//! Each mecha will focus on a single coe symbol at all times, and then auxiliary
//! ones which we are depending on
//! This way we can pass more information about the symbols which are required etc
//! this will be useful for later

use std::sync::Arc;

use crate::agentic::tool::base::Tool;
use crate::agentic::tool::errors::ToolError;
use crate::agentic::tool::input::ToolInput;
use crate::agentic::tool::output::ToolOutput;
use crate::{
    agentic::tool::broker::ToolBroker, chunking::text_document::Range,
    inline_completion::symbols_tracker::SymbolTrackerInline, user_context::types::UserContext,
};

use super::events::input::MechaInputEvent;

#[derive(Debug, Clone)]
enum MechaState {
    NoSymbol,
    Exploring,
    Editing,
    Fixing,
    Completed,
}

struct Snippet {
    range: Range,
    fs_file_path: String,
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
enum MechaEvent {
    InitialRequest(MechaInputEvent),
}

struct MechaBasic {
    symbol_name: Option<MechaSymbol>,
    history_symbols: Vec<String>,
    current_query: String,
    // Lets keep it this way so we can pass a trace of all we have done
    interactions: Vec<String>,
    state: MechaState,
    tools: Arc<ToolBroker>,
    symbol_broker: Arc<SymbolTrackerInline>,
}

impl MechaBasic {
    pub fn new(
        user_query: String,
        tools: Arc<ToolBroker>,
        symbol_broker: Arc<SymbolTrackerInline>,
    ) -> Self {
        Self {
            symbol_name: None,
            history_symbols: Vec::new(),
            current_query: user_query,
            interactions: Vec::new(),
            state: MechaState::Exploring,
            tools,
            symbol_broker,
        }
    }

    // we need a function here which will just call tools and move between
    // states and maybe even spawn new mechas at some point, the goal is that
    // we only focus on a single symbol at a time
    // how do we go about designing that, lets start with a loop and see how well
    // we can do
    pub async fn get_tool(&mut self, event: MechaEvent) -> Option<ToolInput> {
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

    // Now we have tha basic iteration loop setup, we know this is bad but this
    // is enough to get started
    // once we have the tool output, we act on it over here and wait for our
    // next iteration to start
    pub async fn iterate(mut self, event: MechaEvent) -> Result<ToolOutput, ToolError> {
        let tool = self.get_tool(event).await;
        if let Some(tool) = tool {
            self.tools.invoke(tool).await
        } else {
            Err(ToolError::MissingTool)
        }
    }
}
