//! This contains the input context and how we want to execute action on top of it, we are able to
//! convert between different types of inputs.. something like that
//! or we can keep hardcoded actions somewhere.. we will figure it out as we go

use std::{path::Path, sync::Arc};

use llm_client::{
    clients::types::LLMType,
    provider::{GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys},
};

use crate::{
    agentic::{
        symbol::{identifier::LLMProperties, tool_box::ToolBox},
        tool::{
            code_symbol::{
                important::CodeSymbolImportantWideSearch, repo_map_search::RepoMapSearchQuery,
            },
            file::file_finder::ImportantFilesFinderQuery,
            input::ToolInput,
        },
    },
    tree_printer::tree::TreePrinter,
    user_context::types::UserContext,
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct SymbolInputEvent {
    context: UserContext,
    llm: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    user_query: String,
    request_id: String,
    // Here we have properties for swe bench which we are sending for testing
    swe_bench_test_endpoint: Option<String>,
    repo_map_fs_path: Option<String>,
    gcloud_access_token: Option<String>,
    swe_bench_id: Option<String>,
    swe_bench_git_dname: Option<String>,
    swe_bench_code_editing: Option<LLMProperties>,
    swe_bench_gemini_api_keys: Option<LLMProperties>,
    swe_bench_long_context_editing: Option<LLMProperties>,
    full_symbol_edit: bool,
    codebase_search: bool,
    root_directory: Option<String>,
    /// The properties for the llm which does fast and stable
    /// code symbol selection on an initial context, this can be used
    /// when we are not using full codebase context search
    fast_code_symbol_search_llm: Option<LLMProperties>,
    file_important_search: bool, // todo: this currently conflicts with repomap search
}

impl SymbolInputEvent {
    pub fn new(
        context: UserContext,
        llm: LLMType,
        provider: LLMProvider,
        api_keys: LLMProviderAPIKeys,
        user_query: String,
        request_id: String,
        swe_bench_test_endpoint: Option<String>,
        repo_map_fs_path: Option<String>,
        gcloud_access_token: Option<String>,
        swe_bench_id: Option<String>,
        swe_bench_git_dname: Option<String>,
        swe_bench_code_editing: Option<LLMProperties>,
        swe_bench_gemini_api_keys: Option<LLMProperties>,
        swe_bench_long_context_editing: Option<LLMProperties>,
        full_symbol_edit: bool,
        codebase_search: bool,
        root_directory: Option<String>,
        fast_code_symbol_search_llm: Option<LLMProperties>,
        file_important_search: bool,
    ) -> Self {
        Self {
            context,
            llm,
            provider,
            api_keys,
            request_id,
            user_query,
            swe_bench_test_endpoint,
            repo_map_fs_path,
            gcloud_access_token,
            swe_bench_id,
            swe_bench_git_dname,
            swe_bench_code_editing,
            swe_bench_gemini_api_keys,
            swe_bench_long_context_editing,
            full_symbol_edit,
            codebase_search,
            root_directory,
            fast_code_symbol_search_llm,
            file_important_search,
        }
    }

    pub fn full_symbol_edit(&self) -> bool {
        self.full_symbol_edit
    }

    pub fn user_query(&self) -> &str {
        &self.user_query
    }

    pub fn get_swe_bench_git_dname(&self) -> Option<String> {
        self.swe_bench_git_dname.clone()
    }

    pub fn get_swe_bench_test_endpoint(&self) -> Option<String> {
        self.swe_bench_test_endpoint.clone()
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

    pub fn get_fast_code_symbol_llm(&self) -> Option<LLMProperties> {
        self.fast_code_symbol_search_llm.clone()
    }

    pub fn get_swe_bench_code_editing(&self) -> Option<LLMProperties> {
        self.swe_bench_code_editing.clone()
    }

    pub fn get_swe_bench_gemini_llm_properties(&self) -> Option<LLMProperties> {
        self.swe_bench_gemini_api_keys.clone()
    }

    pub fn get_swe_bench_long_context_editing(&self) -> Option<LLMProperties> {
        self.swe_bench_long_context_editing.clone()
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    // here we can take an action based on the state we are in
    // on some states this might be wrong, I find it a bit easier to reason
    // altho fuck complexity we ball
    pub async fn tool_use_on_initial_invocation(
        self,
        tool_box: Arc<ToolBox>,
        request_id: &str,
    ) -> Option<ToolInput> {
        // if its anthropic we purposefully override the llm here to be a better
        // model (if they are using their own api-keys and even the codestory provider)
        let llm_properties_for_symbol_search =
            if let Some(llm_properties) = self.get_fast_code_symbol_llm() {
                llm_properties.clone()
            } else {
                LLMProperties::new(
                    self.llm.clone(),
                    self.provider.clone(),
                    self.api_keys.clone(),
                )
            };
        // TODO(skcd): Toggle the request here depending on if we have the repo map
        if self.has_repo_map() || self.root_directory.is_some() {
            let contents = if self.has_repo_map() {
                tokio::fs::read_to_string(self.repo_map_fs_path.expect("has_repo_map to not break"))
                    .await
                    .ok()
            } else {
                None
            };
            match contents {
                Some(contents) => Some(ToolInput::RepoMapSearch(RepoMapSearchQuery::new(
                    contents,
                    self.user_query.to_owned(),
                    LLMType::ClaudeSonnet,
                    LLMProvider::Anthropic,
                    self.api_keys.clone(),
                    None,
                    self.request_id.to_string(),
                ))),
                None => {
                    // try to fetch it from the root_directory using repo_search
                    if let Some(root_directory) = self.root_directory.to_owned() {
                        if self.file_important_search {
                            let dir = Path::new(&root_directory);

                            let repo_name = "sidecar";

                            let api_key =
                                LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                                    "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
                                ));

                            let (tree, _, _) = TreePrinter::to_string(dir).unwrap();

                            println!("{}", tree);

                            let llm_type = LLMType::GeminiProFlash;

                            let finder_query = ImportantFilesFinderQuery::new(
                                tree,
                                self.user_query.to_owned(),
                                llm_type,
                                LLMProvider::GoogleAIStudio,
                                api_key,
                                repo_name.to_owned(),
                                "".to_owned(),
                            );

                            return Some(ToolInput::ImportantFilesFinder(finder_query));
                        }
                        if self.codebase_search {
                            // here, search tool, repomap plus files
                            println!("symbol_input::load_repo_map::start({})", &request_id);
                            return tool_box
                                .load_repo_map(&root_directory, request_id)
                                .await
                                .map(|repo_map| {
                                    ToolInput::RepoMapSearch(RepoMapSearchQuery::new(
                                        repo_map,
                                        self.user_query.to_owned(),
                                        LLMType::GeminiProFlash,
                                        LLMProvider::GoogleAIStudio,
                                        LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                                            "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
                                        )),
                                        Some(root_directory.to_owned()),
                                        self.request_id.to_string(),
                                    ))
                                });
                        }
                    }
                    let outline_for_user_context = tool_box
                        .outline_for_user_context(&self.context, &self.request_id)
                        .await;
                    let code_wide_search: CodeSymbolImportantWideSearch =
                        CodeSymbolImportantWideSearch::new(
                            self.context,
                            self.user_query.to_owned(),
                            llm_properties_for_symbol_search.llm().clone(),
                            llm_properties_for_symbol_search.provider().clone(),
                            llm_properties_for_symbol_search.api_key().clone(),
                            self.request_id.to_string(),
                            outline_for_user_context,
                        );
                    // just symbol search instead for quick access
                    return Some(ToolInput::RequestImportantSymbolsCodeWide(code_wide_search));
                }
            }
        } else {
            let outline_for_user_context = tool_box
                .outline_for_user_context(&self.context, &self.request_id)
                .await;
            let code_wide_search: CodeSymbolImportantWideSearch =
                CodeSymbolImportantWideSearch::new(
                    self.context,
                    self.user_query.to_owned(),
                    llm_properties_for_symbol_search.llm().clone(),
                    llm_properties_for_symbol_search.provider().clone(),
                    llm_properties_for_symbol_search.api_key().clone(),
                    self.request_id.to_string(),
                    outline_for_user_context,
                );
            // Now we try to generate the tool input for this
            Some(ToolInput::RequestImportantSymbolsCodeWide(code_wide_search))
        }
    }
}
