//! This contains the input context and how we want to execute action on top of it, we are able to
//! convert between different types of inputs.. something like that
//! or we can keep hardcoded actions somewhere.. we will figure it out as we go

use llm_client::{
    clients::types::LLMType,
    provider::{GeminiProAPIKey, LLMProvider, LLMProviderAPIKeys},
};

use crate::{
    agentic::tool::{
        code_symbol::{
            important::CodeSymbolImportantWideSearch,
            repo_map_search::{RepoMapSearch, RepoMapSearchQuery},
        },
        input::ToolInput,
    },
    user_context::types::UserContext,
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct SymbolInputEvent {
    context: UserContext,
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    user_query: String,
    // Here we have properties for swe bench which we are sending for testing
    swe_bench_test_endpoint: Option<String>,
    repo_map_fs_path: Option<String>,
    gcloud_access_token: Option<String>,
    swe_bench_id: Option<String>,
    swe_bench_git_dname: Option<String>,
}

impl SymbolInputEvent {
    pub fn new(
        context: UserContext,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        user_query: String,
        swe_bench_test_endpoint: Option<String>,
        repo_map_fs_path: Option<String>,
        gcloud_access_token: Option<String>,
        swe_bench_id: Option<String>,
        swe_bench_git_dname: Option<String>,
    ) -> Self {
        Self {
            context,
            llm,
            provider,
            api_keys,
            user_query,
            swe_bench_test_endpoint,
            repo_map_fs_path,
            gcloud_access_token,
            swe_bench_id,
            swe_bench_git_dname,
        }
    }

    pub fn get_swe_bench_git_dname(&self) -> Option<String> {
        self.swe_bench_git_dname.clone()
    }

    pub fn set_swe_bench_id(mut self, swe_bench_id: String) -> Self {
        self.swe_bench_id = Some(swe_bench_id);
        self
    }

    pub fn swe_bench_instance_id(&self) -> Option<String> {
        self.swe_bench_id.clone()
    }

    pub fn provided_context(&self) -> &UserContext {
        &self.context
    }

    pub fn has_repo_map(&self) -> bool {
        self.repo_map_fs_path.is_some()
    }

    // here we can take an action based on the state we are in
    // on some states this might be wrong, I find it a bit easier to reason
    // altho fuck complexity we ball
    pub async fn tool_use_on_initial_invocation(self) -> Option<ToolInput> {
        // if its anthropic we purposefully override the llm here to be a better
        // model (if they are using their own api-keys and even the codestory provider)
        let final_model = if self.llm.is_anthropic()
            && (self.provider.is_codestory() || self.provider.is_anthropic_api_key())
        {
            LLMType::ClaudeSonnet
        } else {
            self.llm.clone()
        };
        // TODO(skcd): Toggle the request here depending on if we have the repo map
        if self.has_repo_map() {
            let contents = tokio::fs::read_to_string(
                self.repo_map_fs_path.expect("has_repo_map to not break"),
            )
            .await;
            println!("repo_map_contents: {}", contents.is_ok());
            match contents {
                Ok(contents) => Some(ToolInput::RepoMapSearch(RepoMapSearchQuery::new(
                    contents,
                    self.user_query.to_owned(),
                    LLMType::GeminiProFlash,
                    LLMProvider::GeminiPro,
                    LLMProviderAPIKeys::GeminiPro(GeminiProAPIKey::new(
                        self.gcloud_access_token
                            .expect("swe bench harness always sends this"),
                        "anton-390822".to_owned(),
                    )),
                ))),
                Err(_) => None,
            }
        } else {
            let code_wide_search: CodeSymbolImportantWideSearch =
                CodeSymbolImportantWideSearch::new(
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
}
