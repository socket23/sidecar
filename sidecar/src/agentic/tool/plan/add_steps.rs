//! We want to add steps to the plan this allows us to pick up the plan at some point
//! and add more steps if required
//!
//! Open questions: should we even show the rest of the plan, or just the prefix of the plan up until a point

use std::sync::Arc;

use llm_client::broker::LLMBroker;

use crate::{
    agentic::tool::helpers::diff_recent_changes::DiffRecentChanges,
    user_context::types::UserContext,
};

#[derive(Debug, Clone)]
pub struct PlanAddRequest {
    plan_up_until_now: String,
    user_context: UserContext,
    plan_add_query: String,
    recent_edits: DiffRecentChanges,
    editor_url: String,
    root_request_id: String,
}

impl PlanAddRequest {
    pub fn new(
        plan_up_until_now: String,
        user_context: UserContext,
        plan_add_query: String,
        recent_edits: DiffRecentChanges,
        editor_url: String,
        root_request_id: String,
    ) -> Self {
        Self {
            plan_up_until_now,
            user_context,
            plan_add_query,
            recent_edits,
            editor_url,
            root_request_id,
        }
    }
}

pub struct PlanAddStepClient {
    llm_client: Arc<LLMBroker>,
}

impl PlanAddStepClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    fn system_message(&self) -> String {
        format!(
            r#"You are an expert software engineer working alongside a developer, you take the user query and add the minimum number of steps to the plan to make sure that it satisfies the new user query.
- The previous part of the plan has already been executed, so we can not go back on that, we can only perform new operations.
- You are provided with the following information, use this to understand the reasoning of the changes and how to help the user.
- <initial_query> This is the initial user query for which we have generated and executed the plan.
- <plan_executed_until_now> This is the plan which we have executed until now.
- <recent_edits> These are the recent edits which we have made to the codebase already.
- <user_context> This is the context the user has provided.
- <user_current_query> This is the CURRENT USER QUERY which we want to add steps for."#
        )
    }

    /// Think of cache hits over here, whats the best way to get this?
    /// 
    /// We want to create the update message over here and get the output in the same format
    /// For some reason this is not a core construct of ours which is weird, we should work on a structure
    /// for prompt and always parse it accordingly
    fn user_message(&self, context: PlanAddRequest) -> String {
        format!(r#""#)
    }
}
