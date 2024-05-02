//! Contains actions which are invoked by either the AI or the huamn
//! Each action can:
//! depend on another agent
//! - Can use multiple-tool
//! - Can ask the user for help if required
//! - Has a memory for context
//! - Can effect the environment and lead to other sub-actions which need to happen
mod graph;
mod types;
