//! Creates the service which handles saving the session and extending it

use std::sync::Arc;

use crate::agentic::symbol::{manager::SymbolManager, tool_box::ToolBox};

/// The session service which takes care of creating the session and manages the storage
pub struct SessionService {
    tool_box: Arc<ToolBox>,
    symbol_manager: Arc<SymbolManager>,
}

impl SessionService {
    pub fn new(tool_box: Arc<ToolBox>, symbol_manager: Arc<SymbolManager>) -> Self {
        Self {
            tool_box,
            symbol_manager,
        }
    }
}
