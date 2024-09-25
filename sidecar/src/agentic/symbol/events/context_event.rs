//! Contains all the different kind of events which we get while getting a context
//! recording from the user
//! This helps the user interact with the editor in a very natural way and for the agent
//! to understand the different steps the user has taken to get a task done

use crate::chunking::text_document::{Position, Range};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpenFileContextEvent {
    fs_file_path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LSPContextEventDestination {
    fs_file_path: String,
    position: Position,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LSPContextEvent {
    fs_file_path: String,
    position: Position,
    event_type: String,
    destination: Option<LSPContextEventDestination>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelectionContextEvent {
    fs_file_path: String,
    range: Range,
}

/// All the context-driven events which can happen in the editor that are useful
/// and done by the user in a quest to provide additional context to the agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ContextGatheringEvent {
    OpenFile(OpenFileContextEvent),
    LSPContextEvent(LSPContextEvent),
    Selection(SelectionContextEvent),
}
