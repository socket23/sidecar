//! This contains the input context and how we want to execute action on top of it, we are able to
//! convert between different types of inputs.. something like that
//! or we can keep hardcoded actions somewhere.. we will figure it out as we go

use llm_client::{
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::{
    agentic::tool::{code_symbol::important::CodeSymbolImportantWideSearch, input::ToolInput},
    user_context::types::UserContext,
};

pub struct SymbolInputEvent {
    context: UserContext,
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    user_query: String,
}

impl SymbolInputEvent {
    pub fn new(
        context: UserContext,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        user_query: String,
    ) -> Self {
        Self {
            context,
            llm,
            provider,
            api_keys,
            user_query,
        }
    }

    pub fn provided_context(&self) -> &UserContext {
        &self.context
    }

    // here we can take an action based on the state we are in
    // on some states this might be wrong, I find it a bit easier to reason
    // altho fuck complexity we ball
    pub fn tool_use_on_initial_invocation(self) -> Option<ToolInput> {
        // if its anthropic we purposefully override the llm here to be a better
        // model (if they are using their own api-keys and even the codestory provider)
        let final_model = if self.llm.is_anthropic()
            && (self.provider.is_codestory() || self.provider.is_anthropic_api_key())
        {
            LLMType::ClaudeSonnet
        } else {
            self.llm.clone()
        };
        let code_wide_search: CodeSymbolImportantWideSearch = CodeSymbolImportantWideSearch::new(
            self.context,
            self.user_query.to_owned(),
            final_model,
            self.provider,
            self.api_keys,
        );
        // Now we try to generate the tool input for this
        Some(ToolInput::RequestImportantSybmolsCodeWide(code_wide_search))
    }
}
