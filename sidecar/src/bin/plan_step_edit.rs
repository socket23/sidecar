use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};
use sidecar::{
    agentic::{
        symbol::{
            events::{
                input::{SymbolEventRequestId, SymbolInputEvent},
                message_event::SymbolEventMessageProperties,
            },
            identifier::LLMProperties,
            manager::SymbolManager,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
            input::ToolInput,
            plan::{plan::Plan, plan_step::PlanStep, updater::PlanUpdateRequest},
            r#type::Tool,
        },
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    user_context::types::UserContext,
};
use uuid::Uuid;

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    let request_id = uuid::Uuid::new_v4();
    let request_id_str = request_id.to_string();
    let parea_url = format!(
        r#"https://app.parea.ai/logs?colViz=%7B%220%22%3Afalse%2C%221%22%3Afalse%2C%222%22%3Afalse%2C%223%22%3Afalse%2C%22error%22%3Afalse%2C%22deployment_id%22%3Afalse%2C%22feedback_score%22%3Afalse%2C%22time_to_first_token%22%3Afalse%2C%22scores%22%3Afalse%2C%22start_timestamp%22%3Afalse%2C%22user%22%3Afalse%2C%22session_id%22%3Afalse%2C%22target%22%3Afalse%2C%22experiment_uuid%22%3Afalse%2C%22dataset_references%22%3Afalse%2C%22in_dataset%22%3Afalse%2C%22event_type%22%3Afalse%2C%22request_type%22%3Afalse%2C%22evaluation_metric_names%22%3Afalse%2C%22request%22%3Afalse%2C%22calling_node%22%3Afalse%2C%22edges%22%3Afalse%2C%22metadata_evaluation_metric_names%22%3Afalse%2C%22metadata_event_type%22%3Afalse%2C%22metadata_0%22%3Afalse%2C%22metadata_calling_node%22%3Afalse%2C%22metadata_edges%22%3Afalse%2C%22metadata_root_id%22%3Afalse%7D&filter=%7B%22filter_field%22%3A%22meta_data%22%2C%22filter_operator%22%3A%22equals%22%2C%22filter_key%22%3A%22root_id%22%2C%22filter_value%22%3A%22{request_id_str}%22%7D&page=1&page_size=50&time_filter=1m"#
    );
    println!("===========================================\nRequest ID: {}\nParea AI: {}\n===========================================", request_id.to_string(), parea_url);
    let editor_url = "http://localhost:42425".to_owned();
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
    let anthropic_llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys.clone(),
    );
    let editor_parsing = Arc::new(EditorParsing::default());
    let symbol_broker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));
    let tool_broker = Arc::new(ToolBroker::new(
        Arc::new(
            LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
                .await
                .expect("to initialize properly"),
        ),
        Arc::new(CodeEditBroker::new()),
        symbol_broker.clone(),
        Arc::new(TSLanguageParsing::init()),
        // for our testing workflow we want to apply the edits directly
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::Gpt4O,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
                "sk-proj-BLaSMsWvoO6FyNwo9syqT3BlbkFJo3yqCyKAxWXLm4AvePtt".to_owned(),
            )),
        ),
    ));

    let user_context = UserContext::new(vec![], vec![], None, vec![]);

    let (sender, mut _receiver) = tokio::sync::mpsc::unbounded_channel();

    let _event_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new("".to_owned(), "".to_owned()),
        sender.clone(),
        editor_url.to_owned(),
        tokio_util::sync::CancellationToken::new(),
    );

    let _symbol_manager = SymbolManager::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing,
        anthropic_llm_properties.clone(),
    );

    let problem_statement = "add a new field user_id to the Tag struct".to_owned();

    let root_dir = "/Users/zi/codestory/sidecar/sidecar/src";

    let _initial_request = SymbolInputEvent::new(
        user_context,
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys,
        problem_statement,
        request_id.to_string(),
        request_id.to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        true, // full code editing
        Some(root_dir.to_string()),
        None,
        true, // big_search
        sender,
    );

    let _initial_context = r##"use super::{
    code_edit::{
        filter_edit::FilterEditOperationRequest, find::FindCodeSelectionInput,
        search_and_replace::SearchAndReplaceEditingRequest,
        test_correction::TestOutputCorrectionRequest, types::CodeEdit,
    },
    code_symbol::{
        apply_outline_edit_to_range::ApplyOutlineEditsToRangeRequest,
        correctness::CodeCorrectnessRequest,
        error_fix::CodeEditingErrorRequest,
        find_file_for_new_symbol::FindFileForSymbolRequest,
        find_symbols_to_edit_in_context::FindSymbolsToEditInContextRequest,
        followup::ClassSymbolFollowupRequest,
        important::{
            CodeSymbolFollowAlongForProbing, CodeSymbolImportantRequest,
            CodeSymbolImportantWideSearch, CodeSymbolProbingSummarize,
            CodeSymbolToAskQuestionsRequest, CodeSymbolUtilityRequest,
        },
        initial_request_follow::CodeSymbolFollowInitialRequest,
        new_location::CodeSymbolNewLocationRequest,
        new_sub_symbol::NewSubSymbolRequiredRequest,
        planning_before_code_edit::PlanningBeforeCodeEditRequest,
        probe::ProbeEnoughOrDeeperRequest,
        probe_question_for_symbol::ProbeQuestionForSymbolRequest,
        probe_try_hard_answer::ProbeTryHardAnswerSymbolRequest,
        repo_map_search::RepoMapSearchQuery,
        reranking_symbols_for_editing_context::ReRankingSnippetsForCodeEditingRequest,
        scratch_pad::ScratchPadAgentInput,
        should_edit::ShouldEditCodeSymbolRequest,
    },
    editor::apply::EditorApplyRequest,
    errors::ToolError,
    file::file_finder::ImportantFilesFinderQuery,
    filtering::broker::{
        CodeToEditFilterRequest, CodeToEditSymbolRequest, CodeToProbeSubSymbolRequest,
    },
    git::{diff_client::GitDiffClientRequest, edited_files::EditedFilesRequest},
    grep::file::FindInFileRequest,
    kw_search::tool::KeywordSearchQuery,
    lsp::{
        diagnostics::LSPDiagnosticsInput,
        get_outline_nodes::OutlineNodesUsingEditorRequest,
        gotodefintion::GoToDefinitionRequest,
        gotoimplementations::GoToImplementationRequest,
        gotoreferences::GoToReferencesRequest,
        grep_symbol::LSPGrepSymbolInCodebaseRequest,
        inlay_hints::InlayHintsRequest,
        open_file::OpenFileRequest,
        quick_fix::{GetQuickFixRequest, LSPQuickFixInvocationRequest},
    },
    plan::{reasoning::ReasoningRequest, updater::PlanUpdateRequest},
    r#type::ToolType,
    ref_filter::ref_filter::ReferenceFilterRequest,
    rerank::base::ReRankEntriesForBroker,
    search::big_search::BigSearchRequest,
    swe_bench::test_tool::SWEBenchTestRequest,
};

#[derive(Debug, Clone)]
pub enum ToolInput {
    CodeEditing(CodeEdit),
    LSPDiagnostics(LSPDiagnosticsInput),
    FindCodeSnippets(FindCodeSelectionInput),
    ReRank(ReRankEntriesForBroker),
    CodeSymbolUtilitySearch(CodeSymbolUtilityRequest),
    RequestImportantSymbols(CodeSymbolImportantRequest),
    RequestImportantSymbolsCodeWide(CodeSymbolImportantWideSearch),
    GoToDefinition(GoToDefinitionRequest),
    GoToReference(GoToReferencesRequest),
    OpenFile(OpenFileRequest),
    GrepSingleFile(FindInFileRequest),
    SymbolImplementations(GoToImplementationRequest),
    FilterCodeSnippetsForEditing(CodeToEditFilterRequest),
    FilterCodeSnippetsForEditingSingleSymbols(CodeToEditSymbolRequest),
    EditorApplyChange(EditorApplyRequest),
    QuickFixRequest(GetQuickFixRequest),
    QuickFixInvocationRequest(LSPQuickFixInvocationRequest),
    CodeCorrectnessAction(CodeCorrectnessRequest),
    CodeEditingError(CodeEditingErrorRequest),
    ClassSymbolFollowup(ClassSymbolFollowupRequest),
    // probe request
    ProbeCreateQuestionForSymbol(ProbeQuestionForSymbolRequest),
    ProbeEnoughOrDeeper(ProbeEnoughOrDeeperRequest),
    ProbeFilterSnippetsSingleSymbol(CodeToProbeSubSymbolRequest),
    ProbeSubSymbol(CodeToEditFilterRequest),
    ProbePossibleRequest(CodeSymbolToAskQuestionsRequest),
    ProbeQuestionAskRequest(CodeSymbolToAskQuestionsRequest),
    ProbeFollowAlongSymbol(CodeSymbolFollowAlongForProbing),
    ProbeSummarizeAnswerRequest(CodeSymbolProbingSummarize),
    ProbeTryHardAnswerRequest(ProbeTryHardAnswerSymbolRequest),
    // repo map query
    RepoMapSearch(RepoMapSearchQuery),
    // important files query
    ImportantFilesFinder(ImportantFilesFinderQuery),
    // SWE Bench tooling
    SWEBenchTest(SWEBenchTestRequest),
    // Test output correction
    TestOutputCorrection(TestOutputCorrectionRequest),
    // Code symbol follow initial request
    CodeSymbolFollowInitialRequest(CodeSymbolFollowInitialRequest),
    // Plan before code editing
    PlanningBeforeCodeEdit(PlanningBeforeCodeEditRequest),
    // New symbols required for code editing
    NewSubSymbolForCodeEditing(NewSubSymbolRequiredRequest),
    // Find the symbol in the codebase which we want to select, this only
    // takes a string as input
    GrepSymbolInCodebase(LSPGrepSymbolInCodebaseRequest),
    // Find file location for the new symbol
    FindFileForNewSymbol(FindFileForSymbolRequest),
    // Find symbol to edit in user context
    FindSymbolsToEditInContext(FindSymbolsToEditInContextRequest),
    // ReRanking outline nodes for code editing context
    ReRankingCodeSnippetsForEditing(ReRankingSnippetsForCodeEditingRequest),
    // Apply the generated code outline to the range we are interested in
    ApplyOutlineEditToRange(ApplyOutlineEditsToRangeRequest),
    // Big search
    BigSearch(BigSearchRequest),
    // checks if the edit operation needs to be performed or is an extra
    FilterEditOperation(FilterEditOperationRequest),
    // Keyword search
    KeywordSearch(KeywordSearchQuery),
    // inlay hints from the lsp/editor
    InlayHints(InlayHintsRequest),
    CodeSymbolNewLocation(CodeSymbolNewLocationRequest),
    // should edit the code symbol
    ShouldEditCode(ShouldEditCodeSymbolRequest),
    // search and replace blocks
    SearchAndReplaceEditing(SearchAndReplaceEditingRequest),
    // git diff request
    GitDiff(GitDiffClientRequest),
    OutlineNodesUsingEditor(OutlineNodesUsingEditorRequest),
    // filters references based on user query
    ReferencesFilter(ReferenceFilterRequest),
    // Scratch pad agent input request
    ScratchPadInput(ScratchPadAgentInput),
    // edited files ordered by timestamp
    EditedFiles(EditedFilesRequest),
    // reasoning with just context
    Reasoning(ReasoningRequest),
    // update plan
    UpdatePlan(PlanUpdateRequest),
}

impl ToolInput {
    pub fn tool_type(&self) -> ToolType {
        match self {
            ToolInput::CodeEditing(_) => ToolType::CodeEditing,
            ToolInput::LSPDiagnostics(_) => ToolType::LSPDiagnostics,
            ToolInput::FindCodeSnippets(_) => ToolType::FindCodeSnippets,
            ToolInput::ReRank(_) => ToolType::ReRank,
            ToolInput::RequestImportantSymbols(_) => ToolType::RequestImportantSymbols,
            ToolInput::RequestImportantSymbolsCodeWide(_) => ToolType::FindCodeSymbolsCodeBaseWide,
            ToolInput::GoToDefinition(_) => ToolType::GoToDefinitions,
            ToolInput::GoToReference(_) => ToolType::GoToReferences,
            ToolInput::OpenFile(_) => ToolType::OpenFile,
            ToolInput::GrepSingleFile(_) => ToolType::GrepInFile,
            ToolInput::SymbolImplementations(_) => ToolType::GoToImplementations,
            ToolInput::FilterCodeSnippetsForEditing(_) => ToolType::FilterCodeSnippetsForEditing,
            ToolInput::FilterCodeSnippetsForEditingSingleSymbols(_) => {
                ToolType::FilterCodeSnippetsSingleSymbolForEditing
            }
            ToolInput::EditorApplyChange(_) => ToolType::EditorApplyEdits,
            ToolInput::CodeSymbolUtilitySearch(_) => ToolType::UtilityCodeSymbolSearch,
            ToolInput::QuickFixRequest(_) => ToolType::GetQuickFix,
            ToolInput::QuickFixInvocationRequest(_) => ToolType::ApplyQuickFix,
            ToolInput::CodeCorrectnessAction(_) => ToolType::CodeCorrectnessActionSelection,
            ToolInput::CodeEditingError(_) => ToolType::CodeEditingForError,
            ToolInput::ClassSymbolFollowup(_) => ToolType::ClassSymbolFollowup,
            ToolInput::ProbePossibleRequest(_) => ToolType::ProbePossible,
            ToolInput::ProbeQuestionAskRequest(_) => ToolType::ProbeQuestion,
            ToolInput::ProbeSubSymbol(_) => ToolType::ProbeSubSymbol,
            ToolInput::ProbeFollowAlongSymbol(_) => ToolType::ProbeFollowAlongSymbol,
            ToolInput::ProbeSummarizeAnswerRequest(_) => ToolType::ProbeSummarizeAnswer,
            ToolInput::RepoMapSearch(_) => ToolType::RepoMapSearch,
            ToolInput::ImportantFilesFinder(_) => ToolType::ImportantFilesFinder,
            ToolInput::SWEBenchTest(_) => ToolType::SWEBenchToolEndpoint,
            ToolInput::TestOutputCorrection(_) => ToolType::TestCorrection,
            ToolInput::CodeSymbolFollowInitialRequest(_) => {
                ToolType::CodeSymbolsToFollowInitialRequest
            }
            ToolInput::ProbeFilterSnippetsSingleSymbol(_) => ToolType::ProbeSubSymbolFiltering,
            ToolInput::ProbeEnoughOrDeeper(_) => ToolType::ProbeEnoughOrDeeper,
            ToolInput::ProbeCreateQuestionForSymbol(_) => ToolType::ProbeCreateQuestionForSymbol,
            ToolInput::PlanningBeforeCodeEdit(_) => ToolType::PlanningBeforeCodeEdit,
            ToolInput::NewSubSymbolForCodeEditing(_) => ToolType::NewSubSymbolRequired,
            ToolInput::ProbeTryHardAnswerRequest(_) => ToolType::ProbeTryHardAnswer,
            ToolInput::GrepSymbolInCodebase(_) => ToolType::GrepSymbolInCodebase,
            ToolInput::FindFileForNewSymbol(_) => ToolType::FindFileForNewSymbol,
            ToolInput::FindSymbolsToEditInContext(_) => ToolType::FindSymbolsToEditInContext,
            ToolInput::ReRankingCodeSnippetsForEditing(_) => {
                ToolType::ReRankingCodeSnippetsForCodeEditingContext
            }
            ToolInput::ApplyOutlineEditToRange(_) => ToolType::ApplyOutlineEditToRange,
            ToolInput::BigSearch(_) => ToolType::BigSearch,
            ToolInput::FilterEditOperation(_) => ToolType::FilterEditOperation,
            ToolInput::KeywordSearch(_) => ToolType::KeywordSearch,
            ToolInput::InlayHints(_) => ToolType::InLayHints,
            ToolInput::CodeSymbolNewLocation(_) => ToolType::CodeSymbolNewLocation,
            ToolInput::ShouldEditCode(_) => ToolType::ShouldEditCode,
            ToolInput::SearchAndReplaceEditing(_) => ToolType::SearchAndReplaceEditing,
            ToolInput::GitDiff(_) => ToolType::GitDiff,
            ToolInput::OutlineNodesUsingEditor(_) => ToolType::OutlineNodesUsingEditor,
            ToolInput::ReferencesFilter(_) => ToolType::ReferencesFilter,
            ToolInput::ScratchPadInput(_) => ToolType::ScratchPadAgent,
            ToolInput::EditedFiles(_) => ToolType::EditedFiles,
            ToolInput::Reasoning(_) => ToolType::Reasoning,
            ToolInput::UpdatePlan(_) => ToolType::PlanUpdater,
        }
    }

    pub fn should_reasoning(self) -> Result<ReasoningRequest, ToolError> {
        if let ToolInput::Reasoning(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::Reasoning))
        }
    }

    pub fn should_edited_files(self) -> Result<EditedFilesRequest, ToolError> {
        if let ToolInput::EditedFiles(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::EditedFiles))
        }
    }

    pub fn should_scratch_pad_input(self) -> Result<ScratchPadAgentInput, ToolError> {
        if let ToolInput::ScratchPadInput(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ScratchPadAgent))
        }
    }

    pub fn should_outline_nodes_using_editor(
        self,
    ) -> Result<OutlineNodesUsingEditorRequest, ToolError> {
        if let ToolInput::OutlineNodesUsingEditor(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::OutlineNodesUsingEditor))
        }
    }

    pub fn should_git_diff(self) -> Result<GitDiffClientRequest, ToolError> {
        if let ToolInput::GitDiff(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GitDiff))
        }
    }

    pub fn should_search_and_replace_editing(
        self,
    ) -> Result<SearchAndReplaceEditingRequest, ToolError> {
        if let ToolInput::SearchAndReplaceEditing(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::SearchAndReplaceEditing))
        }
    }

    pub fn should_edit_code(self) -> Result<ShouldEditCodeSymbolRequest, ToolError> {
        if let ToolInput::ShouldEditCode(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ShouldEditCode))
        }
    }

    pub fn code_symbol_new_location(self) -> Result<CodeSymbolNewLocationRequest, ToolError> {
        if let ToolInput::CodeSymbolNewLocation(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::CodeSymbolNewLocation))
        }
    }

    pub fn inlay_hints_request(self) -> Result<InlayHintsRequest, ToolError> {
        if let ToolInput::InlayHints(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::InLayHints))
        }
    }

    pub fn filter_edit_operation_request(self) -> Result<FilterEditOperationRequest, ToolError> {
        if let ToolInput::FilterEditOperation(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::FilterEditOperation))
        }
    }

    pub fn filter_references_request(self) -> Result<ReferenceFilterRequest, ToolError> {
        if let ToolInput::ReferencesFilter(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ReferencesFilter))
        }
    }

    pub fn apply_outline_edits_to_range(
        self,
    ) -> Result<ApplyOutlineEditsToRangeRequest, ToolError> {
        if let ToolInput::ApplyOutlineEditToRange(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ApplyOutlineEditToRange))
        }
    }

    pub fn reranking_code_snippets_for_editing_context(
        self,
    ) -> Result<ReRankingSnippetsForCodeEditingRequest, ToolError> {
        if let ToolInput::ReRankingCodeSnippetsForEditing(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::ReRankingCodeSnippetsForCodeEditingContext,
            ))
        }
    }

    pub fn find_symbols_to_edit_in_context(
        self,
    ) -> Result<FindSymbolsToEditInContextRequest, ToolError> {
        if let ToolInput::FindSymbolsToEditInContext(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::FindSymbolsToEditInContext,
            ))
        }
    }

    pub fn find_file_for_new_symbol(self) -> Result<FindFileForSymbolRequest, ToolError> {
        if let ToolInput::FindFileForNewSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::FindFileForNewSymbol))
        }
    }

    pub fn grep_symbol_in_codebase(self) -> Result<LSPGrepSymbolInCodebaseRequest, ToolError> {
        if let ToolInput::GrepSymbolInCodebase(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GrepSymbolInCodebase))
        }
    }

    pub fn get_probe_try_hard_answer_request(
        self,
    ) -> Result<ProbeTryHardAnswerSymbolRequest, ToolError> {
        if let ToolInput::ProbeTryHardAnswerRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeTryHardAnswer))
        }
    }

    pub fn probe_try_hard_answer(request: ProbeTryHardAnswerSymbolRequest) -> Self {
        ToolInput::ProbeTryHardAnswerRequest(request)
    }

    pub fn get_new_sub_symbol_for_code_editing(
        self,
    ) -> Result<NewSubSymbolRequiredRequest, ToolError> {
        if let ToolInput::NewSubSymbolForCodeEditing(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::NewSubSymbolRequired))
        }
    }

    pub fn probe_create_question_for_symbol(request: ProbeQuestionForSymbolRequest) -> Self {
        ToolInput::ProbeCreateQuestionForSymbol(request)
    }

    pub fn get_probe_create_question_for_symbol(
        self,
    ) -> Result<ProbeQuestionForSymbolRequest, ToolError> {
        if let ToolInput::ProbeCreateQuestionForSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::ProbeCreateQuestionForSymbol,
            ))
        }
    }

    pub fn probe_enough_or_deeper(request: ProbeEnoughOrDeeperRequest) -> Self {
        ToolInput::ProbeEnoughOrDeeper(request)
    }

    pub fn get_probe_enough_or_deeper(self) -> Result<ProbeEnoughOrDeeperRequest, ToolError> {
        if let ToolInput::ProbeEnoughOrDeeper(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeEnoughOrDeeper))
        }
    }

    pub fn probe_filter_snippets_single_symbol(request: CodeToProbeSubSymbolRequest) -> Self {
        ToolInput::ProbeFilterSnippetsSingleSymbol(request)
    }

    pub fn is_probe_filter_snippets_single_symbol(&self) -> bool {
        if let ToolInput::ProbeFilterSnippetsSingleSymbol(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_code_symbol_follow_initial_request(
        self,
    ) -> Result<CodeSymbolFollowInitialRequest, ToolError> {
        if let ToolInput::CodeSymbolFollowInitialRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::CodeSymbolsToFollowInitialRequest,
            ))
        }
    }

    pub fn is_test_output(self) -> Result<TestOutputCorrectionRequest, ToolError> {
        if let ToolInput::TestOutputCorrection(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::TestCorrection))
        }
    }

    pub fn is_probe_subsymbol(&self) -> bool {
        if let ToolInput::ProbeSubSymbol(_) = self {
            true
        } else {
            false
        }
    }

    pub fn swe_bench_test(self) -> Result<SWEBenchTestRequest, ToolError> {
        if let ToolInput::SWEBenchTest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::SWEBenchToolEndpoint))
        }
    }

    pub fn repo_map_search_query(self) -> Result<RepoMapSearchQuery, ToolError> {
        if let ToolInput::RepoMapSearch(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::RepoMapSearch))
        }
    }

    pub fn important_files_finder_query(self) -> Result<ImportantFilesFinderQuery, ToolError> {
        if let ToolInput::ImportantFilesFinder(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ImportantFilesFinder))
        }
    }

    pub fn keyword_search_query(self) -> Result<KeywordSearchQuery, ToolError> {
        if let ToolInput::KeywordSearch(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::KeywordSearch))
        }
    }

    pub fn big_search_query(self) -> Result<BigSearchRequest, ToolError> {
        if let ToolInput::BigSearch(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::BigSearch))
        }
    }

    pub fn probe_sub_symbol_filtering(self) -> Result<CodeToProbeSubSymbolRequest, ToolError> {
        if let ToolInput::ProbeFilterSnippetsSingleSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeSubSymbolFiltering))
        }
    }

    pub fn probe_subsymbol(self) -> Result<CodeToEditFilterRequest, ToolError> {
        if let ToolInput::ProbeSubSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeSubSymbol))
        }
    }

    pub fn probe_possible_request(self) -> Result<CodeSymbolToAskQuestionsRequest, ToolError> {
        if let ToolInput::ProbePossibleRequest(output) = self {
            Ok(output)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbePossible))
        }
    }

    pub fn probe_question_request(self) -> Result<CodeSymbolToAskQuestionsRequest, ToolError> {
        if let ToolInput::ProbeQuestionAskRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeQuestion))
        }
    }

    pub fn probe_follow_along_symbol(self) -> Result<CodeSymbolFollowAlongForProbing, ToolError> {
        if let ToolInput::ProbeFollowAlongSymbol(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeFollowAlongSymbol))
        }
    }

    pub fn probe_summarization_request(self) -> Result<CodeSymbolProbingSummarize, ToolError> {
        if let ToolInput::ProbeSummarizeAnswerRequest(response) = self {
            Ok(response)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ProbeSummarizeAnswer))
        }
    }

    pub fn is_probe_summarization_request(&self) -> bool {
        if let ToolInput::ProbeSummarizeAnswerRequest(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_repo_map_search(&self) -> bool {
        if let ToolInput::RepoMapSearch(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_probe_follow_along_symbol_request(&self) -> bool {
        if let ToolInput::ProbeFollowAlongSymbol(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_probe_possible_request(&self) -> bool {
        if let ToolInput::ProbePossibleRequest(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_probe_question(&self) -> bool {
        if let ToolInput::ProbeQuestionAskRequest(_) = self {
            true
        } else {
            false
        }
    }

    pub fn code_editing_error(self) -> Result<CodeEditingErrorRequest, ToolError> {
        if let ToolInput::CodeEditingError(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::CodeEditingForError))
        }
    }

    pub fn code_correctness_action(self) -> Result<CodeCorrectnessRequest, ToolError> {
        if let ToolInput::CodeCorrectnessAction(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::CodeCorrectnessActionSelection,
            ))
        }
    }

    pub fn quick_fix_invocation_request(self) -> Result<LSPQuickFixInvocationRequest, ToolError> {
        if let ToolInput::QuickFixInvocationRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GetQuickFix))
        }
    }

    pub fn quick_fix_request(self) -> Result<GetQuickFixRequest, ToolError> {
        if let ToolInput::QuickFixRequest(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ApplyQuickFix))
        }
    }

    pub fn editor_apply_changes(self) -> Result<EditorApplyRequest, ToolError> {
        if let ToolInput::EditorApplyChange(editor_apply_request) = self {
            Ok(editor_apply_request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::EditorApplyEdits))
        }
    }

    pub fn symbol_implementations(self) -> Result<GoToImplementationRequest, ToolError> {
        if let ToolInput::SymbolImplementations(symbol_implementations) = self {
            Ok(symbol_implementations)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GoToImplementations))
        }
    }

    pub fn reference_request(self) -> Result<GoToReferencesRequest, ToolError> {
        if let ToolInput::GoToReference(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GoToReferences))
        }
    }

    pub fn class_symbol_followup(self) -> Result<ClassSymbolFollowupRequest, ToolError> {
        if let ToolInput::ClassSymbolFollowup(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ClassSymbolFollowup))
        }
    }

    pub fn grep_single_file(self) -> Result<FindInFileRequest, ToolError> {
        if let ToolInput::GrepSingleFile(grep_single_file) = self {
            Ok(grep_single_file)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GrepInFile))
        }
    }

    pub fn is_file_open(self) -> Result<OpenFileRequest, ToolError> {
        if let ToolInput::OpenFile(open_file) = self {
            Ok(open_file)
        } else {
            Err(ToolError::WrongToolInput(ToolType::OpenFile))
        }
    }

    pub fn is_go_to_definition(self) -> Result<GoToDefinitionRequest, ToolError> {
        if let ToolInput::GoToDefinition(definition_request) = self {
            Ok(definition_request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::GoToDefinitions))
        }
    }

    pub fn is_code_edit(self) -> Result<CodeEdit, ToolError> {
        if let ToolInput::CodeEditing(code_edit) = self {
            Ok(code_edit)
        } else {
            Err(ToolError::WrongToolInput(ToolType::CodeEditing))
        }
    }

    pub fn is_lsp_diagnostics(self) -> Result<LSPDiagnosticsInput, ToolError> {
        if let ToolInput::LSPDiagnostics(lsp_diagnostics) = self {
            Ok(lsp_diagnostics)
        } else {
            Err(ToolError::WrongToolInput(ToolType::LSPDiagnostics))
        }
    }

    pub fn is_code_find(self) -> Result<FindCodeSelectionInput, ToolError> {
        if let ToolInput::FindCodeSnippets(find_code_snippets) = self {
            Ok(find_code_snippets)
        } else {
            Err(ToolError::WrongToolInput(ToolType::FindCodeSnippets))
        }
    }

    pub fn is_rerank(self) -> Result<ReRankEntriesForBroker, ToolError> {
        if let ToolInput::ReRank(rerank) = self {
            Ok(rerank)
        } else {
            Err(ToolError::WrongToolInput(ToolType::ReRank))
        }
    }

    pub fn is_utility_code_search(&self) -> bool {
        if let ToolInput::CodeSymbolUtilitySearch(_) = self {
            true
        } else {
            false
        }
    }

    pub fn utility_code_search(self) -> Result<CodeSymbolUtilityRequest, ToolError> {
        if let ToolInput::CodeSymbolUtilitySearch(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::UtilityCodeSymbolSearch))
        }
    }

    pub fn code_symbol_search(
        self,
    ) -> Result<either::Either<CodeSymbolImportantRequest, CodeSymbolImportantWideSearch>, ToolError>
    {
        if let ToolInput::RequestImportantSymbols(request_code_symbol_important) = self {
            Ok(either::Either::Left(request_code_symbol_important))
        } else if let ToolInput::RequestImportantSymbolsCodeWide(request_code_symbol_important) =
            self
        {
            Ok(either::Either::Right(request_code_symbol_important))
        } else {
            Err(ToolError::WrongToolInput(ToolType::UtilityCodeSymbolSearch))
        }
    }

    pub fn filter_code_snippets_for_editing(self) -> Result<CodeToEditFilterRequest, ToolError> {
        if let ToolInput::FilterCodeSnippetsForEditing(filter_code_snippets_for_editing) = self {
            Ok(filter_code_snippets_for_editing)
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::FilterCodeSnippetsForEditing,
            ))
        }
    }

    pub fn filter_code_snippets_request(
        self,
    ) -> Result<either::Either<CodeToEditFilterRequest, CodeToEditSymbolRequest>, ToolError> {
        if let ToolInput::FilterCodeSnippetsForEditing(filter_code_snippets_for_editing) = self {
            Ok(either::Left(filter_code_snippets_for_editing))
        } else if let ToolInput::FilterCodeSnippetsForEditingSingleSymbols(
            filter_code_snippets_for_editing,
        ) = self
        {
            Ok(either::Right(filter_code_snippets_for_editing))
        } else {
            Err(ToolError::WrongToolInput(
                ToolType::FilterCodeSnippetsForEditing,
            ))
        }
    }

    pub fn plan_before_code_editing(self) -> Result<PlanningBeforeCodeEditRequest, ToolError> {
        if let ToolInput::PlanningBeforeCodeEdit(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::PlanningBeforeCodeEdit))
        }
    }

    pub fn plan_updater(self) -> Result<PlanUpdateRequest, ToolError> {
        if let ToolInput::UpdatePlan(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::PlanUpdater))
        }
    }
}


//! Contains the output of a tool which can be used by any of the callers

use crate::agentic::symbol::ui_event::RelevantReference;

use super::{
    code_edit::{
        filter_edit::FilterEditOperationResponse,
        search_and_replace::SearchAndReplaceEditingResponse,
    },
    code_symbol::{
        apply_outline_edit_to_range::ApplyOutlineEditsToRangeResponse,
        correctness::CodeCorrectnessAction,
        find_file_for_new_symbol::FindFileForSymbolResponse,
        find_symbols_to_edit_in_context::FindSymbolsToEditInContextResponse,
        followup::ClassSymbolFollowupResponse,
        important::CodeSymbolImportantResponse,
        initial_request_follow::CodeSymbolFollowInitialResponse,
        models::anthropic::{
            CodeSymbolShouldAskQuestionsResponse, CodeSymbolToAskQuestionsResponse, ProbeNextSymbol,
        },
        new_location::CodeSymbolNewLocationResponse,
        new_sub_symbol::NewSubSymbolRequiredResponse,
        planning_before_code_edit::PlanningBeforeCodeEditResponse,
        probe::ProbeEnoughOrDeeperResponse,
        reranking_symbols_for_editing_context::ReRankingSnippetsForCodeEditingResponse,
        should_edit::ShouldEditCodeSymbolResponse,
    },
    editor::apply::EditorApplyResponse,
    file::important::FileImportantResponse,
    filtering::broker::{
        CodeToEditFilterResponse, CodeToEditSymbolResponse, CodeToProbeFilterResponse,
        CodeToProbeSubSymbolList,
    },
    git::{diff_client::GitDiffClientResponse, edited_files::EditedFilesResponse},
    grep::file::FindInFileResponse,
    lsp::{
        diagnostics::LSPDiagnosticsOutput,
        get_outline_nodes::OutlineNodesUsingEditorResponse,
        gotodefintion::GoToDefinitionResponse,
        gotoimplementations::GoToImplementationResponse,
        gotoreferences::GoToReferencesResponse,
        grep_symbol::LSPGrepSymbolInCodebaseResponse,
        inlay_hints::InlayHintsResponse,
        open_file::OpenFileResponse,
        quick_fix::{GetQuickFixResponse, LSPQuickFixInvocationResponse},
    },
    plan::reasoning::ReasoningResponse,
    rerank::base::ReRankEntriesForBroker,
    swe_bench::test_tool::SWEBenchTestRepsonse,
};

#[derive(Debug)]
pub struct CodeToEditSnippet {
    start_line: i64,
    end_line: i64,
    thinking: String,
}

impl CodeToEditSnippet {
    pub fn start_line(&self) -> i64 {
        self.start_line
    }

    pub fn end_line(&self) -> i64 {
        self.end_line
    }

    pub fn thinking(&self) -> &str {
        &self.thinking
    }
}

#[derive(Debug)]
pub struct CodeToEditToolOutput {
    snipets: Vec<CodeToEditSnippet>,
}

impl CodeToEditToolOutput {
    pub fn new() -> Self {
        CodeToEditToolOutput { snipets: vec![] }
    }

    pub fn add_snippet(&mut self, start_line: i64, end_line: i64, thinking: String) {
        self.snipets.push(CodeToEditSnippet {
            start_line,
            end_line,
            thinking,
        });
    }
}

#[derive(Debug)]
pub enum ToolOutput {
    PlanningBeforeCodeEditing(PlanningBeforeCodeEditResponse),
    CodeEditTool(String),
    LSPDiagnostics(LSPDiagnosticsOutput),
    CodeToEdit(CodeToEditToolOutput),
    ReRankSnippets(ReRankEntriesForBroker),
    ImportantSymbols(CodeSymbolImportantResponse),
    GoToDefinition(GoToDefinitionResponse),
    GoToReference(GoToReferencesResponse),
    FileOpen(OpenFileResponse),
    GrepSingleFile(FindInFileResponse),
    GoToImplementation(GoToImplementationResponse),
    CodeToEditSnippets(CodeToEditFilterResponse),
    CodeToEditSingleSymbolSnippets(CodeToEditSymbolResponse),
    EditorApplyChanges(EditorApplyResponse),
    UtilityCodeSearch(CodeSymbolImportantResponse),
    GetQuickFixList(GetQuickFixResponse),
    LSPQuickFixInvoation(LSPQuickFixInvocationResponse),
    CodeCorrectnessAction(CodeCorrectnessAction),
    CodeEditingForError(String),
    ClassSymbolFollowupResponse(ClassSymbolFollowupResponse),
    // Probe requests
    ProbeCreateQuestionForSymbol(String),
    ProbeEnoughOrDeeper(ProbeEnoughOrDeeperResponse),
    ProbeSubSymbolFiltering(CodeToProbeSubSymbolList),
    ProbePossible(CodeSymbolShouldAskQuestionsResponse),
    ProbeQuestion(CodeSymbolToAskQuestionsResponse),
    ProbeSubSymbol(CodeToProbeFilterResponse),
    ProbeFollowAlongSymbol(ProbeNextSymbol),
    ProbeSummarizationResult(String),
    ProbeTryHardAnswer(String),
    // Repo map result
    RepoMapSearch(CodeSymbolImportantResponse),
    // important files result
    ImportantFilesFinder(FileImportantResponse),
    // Big search result
    BigSearch(CodeSymbolImportantResponse),
    // SWE Bench test output
    SWEBenchTestOutput(SWEBenchTestRepsonse),
    // Test correction output
    TestCorrectionOutput(String),
    // Code Symbol follow for initial request
    CodeSymbolFollowForInitialRequest(CodeSymbolFollowInitialResponse),
    // New sub symbol creation
    NewSubSymbolCreation(NewSubSymbolRequiredResponse),
    // LSP symbol search information
    LSPSymbolSearchInformation(LSPGrepSymbolInCodebaseResponse),
    // Find the file for the symbol
    FindFileForNewSymbol(FindFileForSymbolResponse),
    // Find symbols to edit in the user context
    FindSymbolsToEditInContext(FindSymbolsToEditInContextResponse),
    // the outline nodes which we should use as context for the code editing
    ReRankedCodeSnippetsForCodeEditing(ReRankingSnippetsForCodeEditingResponse),
    // Apply outline edits to the range
    ApplyOutlineEditsToRange(ApplyOutlineEditsToRangeResponse),
    // Filter the edit operations and its reponse
    FilterEditOperation(FilterEditOperationResponse),
    // Keyword search
    KeywordSearch(CodeSymbolImportantResponse),
    // Inlay hints response
    InlayHints(InlayHintsResponse),
    // code symbol new location
    CodeSymbolNewLocation(CodeSymbolNewLocationResponse),
    // should edit the code
    ShouldEditCode(ShouldEditCodeSymbolResponse),
    // search and replace editing
    SearchAndReplaceEditing(SearchAndReplaceEditingResponse),
    // git diff response
    GitDiff(GitDiffClientResponse),
    // outline nodes from the editor
    OutlineNodesUsingEditor(OutlineNodesUsingEditorResponse),
    // filter reference
    ReferencesFilter(Vec<RelevantReference>),
    // edited files with timestamps (git-diff)
    EditedFiles(EditedFilesResponse),
    // reasoning output
    Reasoning(ReasoningResponse),
}

impl ToolOutput {
    pub fn reasoning(response: ReasoningResponse) -> Self {
        ToolOutput::Reasoning(response)
    }

    pub fn edited_files(response: EditedFilesResponse) -> Self {
        ToolOutput::EditedFiles(response)
    }
    pub fn outline_nodes_using_editor(response: OutlineNodesUsingEditorResponse) -> Self {
        ToolOutput::OutlineNodesUsingEditor(response)
    }

    pub fn git_diff_response(response: GitDiffClientResponse) -> Self {
        ToolOutput::GitDiff(response)
    }

    pub fn search_and_replace_editing(response: SearchAndReplaceEditingResponse) -> Self {
        ToolOutput::SearchAndReplaceEditing(response)
    }

    pub fn should_edit_code(response: ShouldEditCodeSymbolResponse) -> Self {
        ToolOutput::ShouldEditCode(response)
    }

    pub fn code_symbol_new_location(response: CodeSymbolNewLocationResponse) -> Self {
        ToolOutput::CodeSymbolNewLocation(response)
    }

    pub fn inlay_hints(response: InlayHintsResponse) -> Self {
        ToolOutput::InlayHints(response)
    }

    pub fn filter_edit_operation(response: FilterEditOperationResponse) -> Self {
        ToolOutput::FilterEditOperation(response)
    }

    pub fn apply_outline_edits_to_range(response: ApplyOutlineEditsToRangeResponse) -> Self {
        ToolOutput::ApplyOutlineEditsToRange(response)
    }

    pub fn re_ranked_code_snippets_for_editing_context(
        response: ReRankingSnippetsForCodeEditingResponse,
    ) -> Self {
        ToolOutput::ReRankedCodeSnippetsForCodeEditing(response)
    }

    pub fn find_symbols_to_edit_in_context(output: FindSymbolsToEditInContextResponse) -> Self {
        ToolOutput::FindSymbolsToEditInContext(output)
    }

    pub fn find_file_for_new_symbol(output: FindFileForSymbolResponse) -> Self {
        ToolOutput::FindFileForNewSymbol(output)
    }

    pub fn lsp_symbol_search_information(output: LSPGrepSymbolInCodebaseResponse) -> Self {
        ToolOutput::LSPSymbolSearchInformation(output)
    }

    pub fn new_sub_symbol_creation(output: NewSubSymbolRequiredResponse) -> Self {
        ToolOutput::NewSubSymbolCreation(output)
    }

    pub fn planning_before_code_editing(output: PlanningBeforeCodeEditResponse) -> Self {
        ToolOutput::PlanningBeforeCodeEditing(output)
    }

    pub fn code_symbol_follow_for_initial_request(output: CodeSymbolFollowInitialResponse) -> Self {
        ToolOutput::CodeSymbolFollowForInitialRequest(output)
    }

    pub fn swe_bench_test_output(output: SWEBenchTestRepsonse) -> Self {
        ToolOutput::SWEBenchTestOutput(output)
    }

    pub fn probe_summarization_result(response: String) -> Self {
        ToolOutput::ProbeSummarizationResult(response)
    }

    pub fn probe_follow_along_symbol(response: ProbeNextSymbol) -> Self {
        ToolOutput::ProbeFollowAlongSymbol(response)
    }

    pub fn probe_sub_symbol(response: CodeToProbeFilterResponse) -> Self {
        ToolOutput::ProbeSubSymbol(response)
    }

    pub fn probe_possible(response: CodeSymbolShouldAskQuestionsResponse) -> Self {
        ToolOutput::ProbePossible(response)
    }

    pub fn go_to_reference(refernece: GoToReferencesResponse) -> Self {
        ToolOutput::GoToReference(refernece)
    }

    pub fn code_correctness_action(output: CodeCorrectnessAction) -> Self {
        ToolOutput::CodeCorrectnessAction(output)
    }

    pub fn quick_fix_invocation_result(output: LSPQuickFixInvocationResponse) -> Self {
        ToolOutput::LSPQuickFixInvoation(output)
    }

    pub fn quick_fix_list(output: GetQuickFixResponse) -> Self {
        ToolOutput::GetQuickFixList(output)
    }

    pub fn code_edit_output(output: String) -> Self {
        ToolOutput::CodeEditTool(output)
    }

    pub fn lsp_diagnostics(diagnostics: LSPDiagnosticsOutput) -> Self {
        ToolOutput::LSPDiagnostics(diagnostics)
    }

    pub fn code_snippets_to_edit(output: CodeToEditToolOutput) -> Self {
        ToolOutput::CodeToEdit(output)
    }

    pub fn rerank_entries(reranked_snippets: ReRankEntriesForBroker) -> Self {
        ToolOutput::ReRankSnippets(reranked_snippets)
    }

    pub fn important_symbols(important_symbols: CodeSymbolImportantResponse) -> Self {
        ToolOutput::ImportantSymbols(important_symbols)
    }

    pub fn utility_code_symbols(important_symbols: CodeSymbolImportantResponse) -> Self {
        ToolOutput::UtilityCodeSearch(important_symbols)
    }

    pub fn go_to_definition(go_to_definition: GoToDefinitionResponse) -> Self {
        ToolOutput::GoToDefinition(go_to_definition)
    }

    pub fn file_open(file_open: OpenFileResponse) -> Self {
        ToolOutput::FileOpen(file_open)
    }

    pub fn go_to_implementation(go_to_implementation: GoToImplementationResponse) -> Self {
        ToolOutput::GoToImplementation(go_to_implementation)
    }

    pub fn get_quick_fix_actions(self) -> Option<GetQuickFixResponse> {
        match self {
            ToolOutput::GetQuickFixList(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_lsp_diagnostics(self) -> Option<LSPDiagnosticsOutput> {
        match self {
            ToolOutput::LSPDiagnostics(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_editor_apply_response(self) -> Option<EditorApplyResponse> {
        match self {
            ToolOutput::EditorApplyChanges(output) => Some(output),
            _ => None,
        }
    }

    /// Grabs the output of filter edit operations from the ToolOutput
    pub fn get_filter_edit_operation_output(self) -> Option<FilterEditOperationResponse> {
        match self {
            ToolOutput::FilterEditOperation(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_code_edit_output(self) -> Option<String> {
        match self {
            ToolOutput::CodeEditTool(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_important_symbols(self) -> Option<CodeSymbolImportantResponse> {
        match self {
            ToolOutput::ImportantSymbols(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_file_open_response(self) -> Option<OpenFileResponse> {
        match self {
            ToolOutput::FileOpen(file_open) => Some(file_open),
            _ => None,
        }
    }

    pub fn grep_single_file(self) -> Option<FindInFileResponse> {
        match self {
            ToolOutput::GrepSingleFile(grep_single_file) => Some(grep_single_file),
            _ => None,
        }
    }

    pub fn get_go_to_definition(self) -> Option<GoToDefinitionResponse> {
        match self {
            ToolOutput::GoToDefinition(go_to_definition) => Some(go_to_definition),
            _ => None,
        }
    }

    pub fn get_go_to_implementation(self) -> Option<GoToImplementationResponse> {
        match self {
            ToolOutput::GoToImplementation(result) => Some(result),
            _ => None,
        }
    }

    pub fn code_to_edit_filter(self) -> Option<CodeToEditFilterResponse> {
        match self {
            ToolOutput::CodeToEditSnippets(code_to_edit_filter) => Some(code_to_edit_filter),
            _ => None,
        }
    }

    pub fn code_to_edit_in_symbol(self) -> Option<CodeToEditSymbolResponse> {
        match self {
            ToolOutput::CodeToEditSingleSymbolSnippets(response) => Some(response),
            _ => None,
        }
    }

    pub fn utility_code_search_response(self) -> Option<CodeSymbolImportantResponse> {
        match self {
            ToolOutput::UtilityCodeSearch(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_test_correction_output(self) -> Option<String> {
        match self {
            ToolOutput::TestCorrectionOutput(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_code_correctness_action(self) -> Option<CodeCorrectnessAction> {
        match self {
            ToolOutput::CodeCorrectnessAction(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_quick_fix_invocation_result(self) -> Option<LSPQuickFixInvocationResponse> {
        match self {
            ToolOutput::LSPQuickFixInvoation(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_references(self) -> Option<GoToReferencesResponse> {
        match self {
            ToolOutput::GoToReference(output) => Some(output),
            _ => None,
        }
    }

    pub fn code_editing_for_error_fix(self) -> Option<String> {
        match self {
            ToolOutput::CodeEditingForError(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_swe_bench_test_output(self) -> Option<SWEBenchTestRepsonse> {
        match self {
            ToolOutput::SWEBenchTestOutput(output) => Some(output),
            _ => None,
        }
    }

    pub fn class_symbols_to_followup(self) -> Option<ClassSymbolFollowupResponse> {
        match self {
            ToolOutput::ClassSymbolFollowupResponse(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_probe_summarize_result(self) -> Option<String> {
        match self {
            ToolOutput::ProbeSummarizationResult(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_probe_sub_symbol(self) -> Option<CodeToProbeFilterResponse> {
        match self {
            ToolOutput::ProbeSubSymbol(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_should_probe_symbol(self) -> Option<CodeSymbolShouldAskQuestionsResponse> {
        match self {
            ToolOutput::ProbePossible(request) => Some(request),
            _ => None,
        }
    }

    pub fn get_probe_symbol_deeper(self) -> Option<CodeSymbolToAskQuestionsResponse> {
        match self {
            ToolOutput::ProbeQuestion(request) => Some(request),
            _ => None,
        }
    }

    pub fn get_should_probe_next_symbol(self) -> Option<ProbeNextSymbol> {
        match self {
            ToolOutput::ProbeFollowAlongSymbol(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_code_symbol_follow_for_initial_request(
        self,
    ) -> Option<CodeSymbolFollowInitialResponse> {
        match self {
            ToolOutput::CodeSymbolFollowForInitialRequest(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_code_to_probe_sub_symbol_list(self) -> Option<CodeToProbeSubSymbolList> {
        match self {
            ToolOutput::ProbeSubSymbolFiltering(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_probe_enough_or_deeper(self) -> Option<ProbeEnoughOrDeeperResponse> {
        match self {
            ToolOutput::ProbeEnoughOrDeeper(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_probe_create_question_for_symbol(self) -> Option<String> {
        match self {
            ToolOutput::ProbeCreateQuestionForSymbol(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_plan_before_code_editing(self) -> Option<PlanningBeforeCodeEditResponse> {
        match self {
            ToolOutput::PlanningBeforeCodeEditing(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_new_sub_symbol_required(self) -> Option<NewSubSymbolRequiredResponse> {
        match self {
            ToolOutput::NewSubSymbolCreation(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_probe_try_harder_answer(self) -> Option<String> {
        match self {
            ToolOutput::ProbeTryHardAnswer(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_find_file_for_symbol_response(self) -> Option<FindFileForSymbolResponse> {
        match self {
            ToolOutput::FindFileForNewSymbol(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_lsp_grep_symbols_in_codebase_response(
        self,
    ) -> Option<LSPGrepSymbolInCodebaseResponse> {
        match self {
            ToolOutput::LSPSymbolSearchInformation(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_code_symbols_to_edit_in_context(self) -> Option<FindSymbolsToEditInContextResponse> {
        match self {
            ToolOutput::FindSymbolsToEditInContext(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_apply_edits_to_range_response(self) -> Option<ApplyOutlineEditsToRangeResponse> {
        match self {
            ToolOutput::ApplyOutlineEditsToRange(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_reranked_outline_nodes_for_code_editing(
        self,
    ) -> Option<ReRankingSnippetsForCodeEditingResponse> {
        match self {
            ToolOutput::ReRankedCodeSnippetsForCodeEditing(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_keyword_search_reply(self) -> Option<CodeSymbolImportantResponse> {
        match self {
            ToolOutput::KeywordSearch(reply) => Some(reply),
            _ => None,
        }
    }

    pub fn get_inlay_hints_response(self) -> Option<InlayHintsResponse> {
        match self {
            ToolOutput::InlayHints(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_code_symbol_new_location(self) -> Option<CodeSymbolNewLocationResponse> {
        match self {
            ToolOutput::CodeSymbolNewLocation(response) => Some(response),
            _ => None,
        }
    }

    pub fn should_edit_code_symbol_full(self) -> Option<ShouldEditCodeSymbolResponse> {
        match self {
            ToolOutput::ShouldEditCode(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_search_and_replace_output(self) -> Option<SearchAndReplaceEditingResponse> {
        match self {
            ToolOutput::SearchAndReplaceEditing(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_git_diff_output(self) -> Option<GitDiffClientResponse> {
        match self {
            ToolOutput::GitDiff(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_outline_nodes_from_editor(self) -> Option<OutlineNodesUsingEditorResponse> {
        match self {
            ToolOutput::OutlineNodesUsingEditor(response) => Some(response),
            _ => None,
        }
    }

    pub fn get_relevant_references(self) -> Option<Vec<RelevantReference>> {
        match self {
            ToolOutput::ReferencesFilter(response) => Some(response),
            _ => None,
        }
    }

    pub fn recently_edited_files(self) -> Option<EditedFilesResponse> {
        match self {
            ToolOutput::EditedFiles(response) => Some(response),
            _ => None,
        }
    }

    pub fn reasoning_output(self) -> Option<ReasoningResponse> {
        match self {
            ToolOutput::Reasoning(response) => Some(response),
            _ => None,
        }
    }
}


//! Contains the basic tool and how to extract data from it

use axum::async_trait;

use super::{errors::ToolError, input::ToolInput, output::ToolOutput};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ToolType {
    // AskDocumentation,
    // AskUser,
    PlanningBeforeCodeEdit,
    CodeEditing,
    OpenFile,
    // Search,
    GoToDefinitions,
    GoToReferences,
    // FileSystem,
    // FolderOutline,
    // Terminal,
    LSPDiagnostics,
    ReRank,
    // WebScrape,
    // searches of different kind are over here
    FindCodeSnippets,
    RequestImportantSymbols,
    FindCodeSymbolsCodeBaseWide,
    UtilityCodeSymbolSearch,
    GrepInFile,
    GoToImplementations,
    // filtering queries go here
    FilterCodeSnippetsForEditing,
    FilterCodeSnippetsSingleSymbolForEditing,
    // editor requests
    EditorApplyEdits,
    // quick fix options
    GetQuickFix,
    // apply quick fix
    ApplyQuickFix,
    // Error correction tool selection
    CodeCorrectnessActionSelection,
    CodeEditingForError,
    // Followup decision
    ClassSymbolFollowup,
    // COT chains
    CodeEditingCOT,
    // Probe operation
    ProbeCreateQuestionForSymbol,
    ProbeEnoughOrDeeper,
    ProbeSubSymbolFiltering,
    ProbePossible,
    ProbeQuestion,
    ProbeSubSymbol,
    ProbeFollowAlongSymbol,
    ProbeSummarizeAnswer,
    ProbeTryHardAnswer,
    // Repo map Search
    RepoMapSearch,
    // Get important files by inferring from repo tree
    ImportantFilesFinder,
    // SWE Bench tool endpoint
    SWEBenchToolEndpoint,
    // Test correction
    TestCorrection,
    // Code symbols which we want to follow
    CodeSymbolsToFollowInitialRequest,
    // Tool to use to generate the final probe answer
    ProbeFinalAnswerSummary,
    // New sub symbol in class for code editing
    NewSubSymbolRequired,
    // Find symbol in the codebase using the vscode api
    GrepSymbolInCodebase,
    // Find new symbol file location
    FindFileForNewSymbol,
    // Find symbol to edit in user context
    FindSymbolsToEditInContext,
    // ReRanking code snippets for code editing context
    ReRankingCodeSnippetsForCodeEditingContext,
    // Apply the outline of the changes to the range we are interested in
    ApplyOutlineEditToRange,
    // Big search
    BigSearch,
    // Filter edit operation
    FilterEditOperation,
    // Keyword search
    KeywordSearch,
    // inlay hints for the code
    InLayHints,
    // code location for the new symbol
    CodeSymbolNewLocation,
    // should edit the code or is it just a check
    ShouldEditCode,
    // use search and replace blocks for edits
    SearchAndReplaceEditing,
    // Grabs the git-diff
    GitDiff,
    // code editing warmup tool
    CodeEditingWarmupTool,
    // grab outline nodes using the editor
    OutlineNodesUsingEditor,
    // filters references
    ReferencesFilter,
    // scratch pad agent
    ScratchPadAgent,
    // edited files
    EditedFiles,
    // Reasoning (This is just plain reasoning with no settings right now)
    Reasoning,
    // Plan updater
    PlanUpdater,
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolType::CodeEditing => write!(f, "Code Editing"),
            ToolType::OpenFile => write!(f, "Open File"),
            ToolType::GoToDefinitions => write!(f, "Go To Definitions"),
            ToolType::GoToReferences => write!(f, "Go To References"),
            ToolType::LSPDiagnostics => write!(f, "LSP Diagnostics"),
            ToolType::ReRank => write!(f, "Re-Rank"),
            ToolType::FindCodeSnippets => write!(f, "Find Code Snippets"),
            ToolType::RequestImportantSymbols => write!(f, "Request Important Symbols"),
            ToolType::FindCodeSymbolsCodeBaseWide => write!(f, "Find Code Symbols Code Base Wide"),
            ToolType::UtilityCodeSymbolSearch => write!(f, "Utility Code Symbol Search"),
            ToolType::GrepInFile => write!(f, "Grep In File"),
            ToolType::GoToImplementations => write!(f, "Go To Implementations"),
            ToolType::FilterCodeSnippetsForEditing => write!(f, "Filter Code Snippets For Editing"),
            ToolType::FilterCodeSnippetsSingleSymbolForEditing => {
                write!(f, "Filter Code Snippets Single Symbol For Editing")
            }
            ToolType::EditorApplyEdits => write!(f, "Editor Apply Edits"),
            ToolType::GetQuickFix => write!(f, "Get Quick Fix"),
            ToolType::ApplyQuickFix => write!(f, "Apply Quick Fix"),
            ToolType::CodeCorrectnessActionSelection => {
                write!(f, "Code Correctness Action Selection")
            }
            ToolType::CodeEditingForError => write!(f, "Code Editing For Error"),
            ToolType::ClassSymbolFollowup => write!(f, "Class Symbol Followup"),
            ToolType::ProbePossible => write!(f, "Probe Possible"),
            ToolType::ProbeQuestion => write!(f, "Probe Question"),
            ToolType::ProbeSubSymbol => write!(f, "Probe Sub Symbol"),
            ToolType::ProbeFollowAlongSymbol => write!(f, "Probe Follow Along Symbol"),
            ToolType::ProbeSummarizeAnswer => write!(f, "Probe Summarize Answer"),
            ToolType::RepoMapSearch => write!(f, "Repo Map Search"),
            ToolType::SWEBenchToolEndpoint => write!(f, "SWE Bench Tool Endpoint"),
            ToolType::TestCorrection => write!(f, "Test Correction"),
            ToolType::CodeEditingCOT => write!(f, "Code editing COT"),
            ToolType::CodeSymbolsToFollowInitialRequest => {
                write!(f, "Code Symbols to follow initial request")
            }
            ToolType::ProbeFinalAnswerSummary => write!(f, "Probe final answer summary"),
            ToolType::ProbeSubSymbolFiltering => write!(f, "Probe sub symbol filtering request"),
            ToolType::ProbeEnoughOrDeeper => write!(f, "Probe enough information or go deeper"),
            ToolType::ProbeCreateQuestionForSymbol => write!(f, "Probe create question for symbol"),
            ToolType::PlanningBeforeCodeEdit => write!(f, "Planning before code edit"),
            ToolType::NewSubSymbolRequired => write!(f, "New sub symbol required for code editing"),
            ToolType::ProbeTryHardAnswer => write!(f, "Probe try hard answer"),
            ToolType::GrepSymbolInCodebase => write!(f, "Grep symbol in the codebase"),
            ToolType::FindFileForNewSymbol => write!(f, "Find file for new symbol"),
            ToolType::FindSymbolsToEditInContext => write!(f, "Find Symbols to edit in context"),
            ToolType::ReRankingCodeSnippetsForCodeEditingContext => {
                write!(f, "ReRanking code snippets for code editing")
            }
            ToolType::ApplyOutlineEditToRange => write!(f, "Apply outline edit to range"),
            ToolType::ImportantFilesFinder => write!(f, "Important files finder"),
            ToolType::BigSearch => write!(f, "Big search"),
            ToolType::FilterEditOperation => write!(f, "Filter edit operation"),
            ToolType::KeywordSearch => write!(f, "Keyword search"),
            ToolType::InLayHints => write!(f, "Inlay hints"),
            ToolType::CodeSymbolNewLocation => write!(f, "Code symbol new location"),
            ToolType::ShouldEditCode => write!(f, "Should edit code"),
            ToolType::SearchAndReplaceEditing => write!(f, "Search and replace editing"),
            ToolType::GitDiff => write!(
                f,
                "Gets the git diff output for a certain file, also returns the original version"
            ),
            ToolType::CodeEditingWarmupTool => write!(f, "Code editing warmup tool"),
            ToolType::OutlineNodesUsingEditor => write!(f, "Outline nodes using the editor"),
            ToolType::ReferencesFilter => write!(f, "Filters references"),
            ToolType::ScratchPadAgent => write!(f, "Scratch pad agent"),
            ToolType::EditedFiles => write!(f, "Edited files"),
            ToolType::Reasoning => write!(f, "Reasoning"),
            ToolType::PlanUpdater => write!(f, "Plan Updater"),
        }
    }
}

#[async_trait]
pub trait Tool {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
}


//! Reasoning tool, we just show it all the information we can and ask it for a query
//! to come up with a plan and thats it

use async_trait::async_trait;
use std::sync::Arc;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};

use crate::{
    agentic::{
        symbol::{identifier::LLMProperties, ui_event::EditedCodeStreamingRequest},
        tool::{
            code_edit::search_and_replace::StreamedEditingForEditor, errors::ToolError,
            input::ToolInput, output::ToolOutput, r#type::Tool,
        },
    },
    chunking::text_document::{Position, Range},
};

#[derive(Debug, Clone)]
pub struct ReasoningResponse {
    response: String,
}

impl ReasoningResponse {
    pub fn response(self) -> String {
        self.response
    }
}

#[derive(Debug, Clone)]
pub struct ReasoningRequest {
    user_query: String,
    files_in_selection: String,
    code_in_selection: String,
    lsp_diagnostics: String,
    diff_recent_edits: String,
    root_request_id: String,
    // These 2 are weird and not really required over here, we are using this
    // as a proxy to output the plan to a file path
    plan_output_path: String,
    plan_output_content: String,
    editor_url: String,
}

impl ReasoningRequest {
    pub fn new(
        user_query: String,
        files_in_selection: String,
        code_in_selection: String,
        lsp_diagnostics: String,
        diff_recent_edits: String,
        root_request_id: String,
        plan_output_path: String,
        plan_output_content: String,
        editor_url: String,
    ) -> Self {
        Self {
            user_query,
            files_in_selection,
            code_in_selection,
            lsp_diagnostics,
            diff_recent_edits,
            root_request_id,
            plan_output_path,
            plan_output_content,
            editor_url,
        }
    }
}

pub struct ReasoningClient {
    llm_client: Arc<LLMBroker>,
}

impl ReasoningClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    fn user_message(&self, context: ReasoningRequest) -> String {
        let user_query = context.user_query;
        let files_in_selection = context.files_in_selection;
        let code_in_selection = context.code_in_selection;
        let lsp_diagnostics = context.lsp_diagnostics;
        let diff_recent_edits = context.diff_recent_edits;
        format!(
            r#"<files_in_selection>
{files_in_selection}
</files_in_selection>
<recent_diff_edits>
{diff_recent_edits}
</recent_diff_edits>
<lsp_diagnostics>
{lsp_diagnostics}
</lsp_diagnostics>
<code_in_selection>
{code_in_selection}
</code_in_selection>

I have provided you with the following context:
- <files_in_selection>
These are the files which are present in context that is useful
- <recent_diff_edits>
The recent edits which have been made to the files
- <lsp_diagnostics>
The diagnostic errors which are generated from the Language Server running inside the editor
- <code_in_selection>
These are the code sections which are in our selection

The query I want help with:
{user_query}"#
        )
    }
}

#[async_trait]
impl Tool for ReasoningClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.should_reasoning()?;
        let editor_url = context.editor_url.to_owned();
        let scratch_pad_path = context.plan_output_path.to_owned();
        let scratch_pad_content = context.plan_output_content.to_owned();
        let root_id = context.root_request_id.to_owned();
        let request = LLMClientCompletionRequest::new(
            LLMType::O1Preview,
            vec![LLMClientMessage::user(self.user_message(context))],
            1.0,
            None,
        );
        let llm_properties = LLMProperties::new(
            LLMType::O1Preview,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("sk-GF8nCfhNTszdK_rr96cxH2vNEQw6aLa4V5FhTka80aT3BlbkFJWS6GYYDuNGSDwqjEuZTSDG2R2EYcHPp14mx8DL6HIA".to_owned())),
        );
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let model_str = llm_properties.llm().to_string();
        let response = self
            .llm_client
            .stream_completion(
                llm_properties.api_key().clone(),
                request,
                llm_properties.provider().clone(),
                vec![
                    ("root_id".to_owned(), root_id),
                    (
                        "event_type".to_owned(),
                        format!("reasoning_{}", model_str).to_owned(),
                    ),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await;
        let output = response
            .map(|response| response)
            .map_err(|e| ToolError::LLMClientError(e))?;

        let scratch_pad_range = Range::new(
            Position::new(0, 0, 0),
            Position::new(
                {
                    let lines = scratch_pad_content
                        .lines()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .len();
                    if lines == 0 {
                        0
                    } else {
                        lines - 1
                    }
                },
                1000,
                0,
            ),
        );

        // Now send this over for writing to the LLM
        let edit_request_id = uuid::Uuid::new_v4().to_string();
        let fs_file_path = scratch_pad_path.to_owned();
        let streamed_edit_client = StreamedEditingForEditor::new();
        streamed_edit_client
            .send_edit_event(
                editor_url.to_owned(),
                EditedCodeStreamingRequest::start_edit(
                    edit_request_id.to_owned(),
                    scratch_pad_range.clone(),
                    fs_file_path.to_owned(),
                )
                .set_apply_directly(),
            )
            .await;
        streamed_edit_client
            .send_edit_event(
                editor_url.to_owned(),
                EditedCodeStreamingRequest::delta(
                    edit_request_id.to_owned(),
                    scratch_pad_range.clone(),
                    fs_file_path.to_owned(),
                    "
\n".to_owned(),
                )
                .set_apply_directly(),
            )
            .await;
        let _ = streamed_edit_client
            .send_edit_event(
                editor_url.to_owned(),
                EditedCodeStreamingRequest::delta(
                    edit_request_id.to_owned(),
                    scratch_pad_range.clone(),
                    fs_file_path.to_owned(),
                    output.to_owned(),
                )
                .set_apply_directly(),
            )
            .await;
        let _ = streamed_edit_client
            .send_edit_event(
                editor_url.to_owned(),
                EditedCodeStreamingRequest::delta(
                    edit_request_id.to_owned(),
                    scratch_pad_range.clone(),
                    fs_file_path.to_owned(),
                    "\n
".to_owned(),
                )
                .set_apply_directly(),
            )
            .await;
        let _ = streamed_edit_client
            .send_edit_event(
                editor_url.to_owned(),
                EditedCodeStreamingRequest::end(
                    edit_request_id.to_owned(),
                    scratch_pad_range.clone(),
                    fs_file_path.to_owned(),
                )
                .set_apply_directly(),
            )
            .await;
        Ok(ToolOutput::reasoning(ReasoningResponse {
            response: output,
        }))
    }
}"##.to_string();

    let user_query =
        "Come up with a stepped plan to create a new Tool, similar to ReasoningClient. 
    Pay special attention to the dependencies that need updating in order to accommodate it. 
    Use prior art and examples where possible, following the patterns of the codebase."
            .to_string();

    let steps = vec![
        r#"Step 1: Define a New ToolType Variant
File: tool/type.rs

Add a new variant to the ToolType enum:

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ToolType {
    // ... existing variants ...
    CodeSummarization,
}"#
        .to_string(),
        r#"Step 2: Define a New ToolInput Variant
File: tool/input.rs

Add a new variant to the ToolInput enum:

#[derive(Debug, Clone)]
pub enum ToolInput {
    // ... existing variants ...
    CodeSummarization(CodeSummarizationRequest),
}"#
        .to_string(),
        r#"Step 3: Create CodeSummarizationRequest Struct
File: tool/code_summarization.rs (new file)

Define the input structure for the summarization tool:

#[derive(Debug, Clone)]
pub struct CodeSummarizationRequest {
    pub code: String,
    pub root_request_id: String,
}"#
        .to_string(),
        r#"Step 4: Create CodeSummarizationResponse Struct
File: tool/code_summarization.rs (same file as above)

Define the output structure for the summarization tool:

#[derive(Debug, Clone)]
pub struct CodeSummarizationResponse {
    pub summary: String,
}"#
        .to_string(),
        r#"Step 5: Implement Methods in ToolInput for the New Variant
File: tool/input.rs

Add a method to extract CodeSummarizationRequest:

impl ToolInput {
    // ... existing methods ...

    pub fn should_code_summarization(self) -> Result<CodeSummarizationRequest, ToolError> {
        if let ToolInput::CodeSummarization(request) = self {
            Ok(request)
        } else {
            Err(ToolError::WrongToolInput(ToolType::CodeSummarization))
        }
    }
}"#
        .to_string(),
        r#"Step 6: Implement Methods in ToolOutput for the New Variant
File: tool/output.rs

Add a new variant to the ToolOutput enum:

#[derive(Debug)]
pub enum ToolOutput {
    // ... existing variants ...
    CodeSummarization(CodeSummarizationResponse),
}

Add methods to handle the new variant:

impl ToolOutput {
    // ... existing methods ...

    pub fn code_summarization(response: CodeSummarizationResponse) -> Self {
        ToolOutput::CodeSummarization(response)
    }

    pub fn get_code_summarization(self) -> Option<CodeSummarizationResponse> {
        match self {
            ToolOutput::CodeSummarization(response) => Some(response),
            _ => None,
        }
    }
}"#
        .to_string(),
        r#"Step 7: Implement the Tool Trait for SummaryClient
File: tool/code_summarization.rs

Implement the Tool trait for SummaryClient:

use async_trait::async_trait;
use std::sync::Arc;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};
use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

pub struct SummaryClient {
    llm_client: Arc<LLMBroker>,
}

impl SummaryClient {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    fn user_message(&self, request: &CodeSummarizationRequest) -> String {
        format!(
            "Please provide a concise summary of the following code:\n\n{}",
            request.code
        )
    }
}

#[async_trait]
impl Tool for SummaryClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let request = input.should_code_summarization()?;

        let llm_request = LLMClientCompletionRequest::new(
            LLMType::GPT4,
            vec![LLMClientMessage::user(self.user_message(&request))],
            0.7,
            None,
        );

        let llm_properties = LLMProperties::new(
            LLMType::GPT4,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("your-api-key-here".to_owned())),
        );

        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        let response = self
            .llm_client
            .stream_completion(
                llm_properties.api_key().clone(),
                llm_request,
                llm_properties.provider().clone(),
                vec![("root_id".to_owned(), request.root_request_id.clone())]
                    .into_iter()
                    .collect(),
                sender,
            )
            .await
            .map_err(ToolError::LLMClientError)?;

        Ok(ToolOutput::code_summarization(CodeSummarizationResponse {
            summary: response,
        }))
    }
}"#
        .to_string(),
        r#"Step 8: Handle the New ToolType in Relevant Code
Files to Update:

Any match statements that handle ToolType or ToolInput.
For example, in tool/invoke.rs or wherever tools are dispatched.
Example:

match tool_input.tool_type() {
    // ... existing matches ...
    ToolType::CodeSummarization => {
        let tool = SummaryClient::new(llm_client.clone());
        tool.invoke(tool_input).await
    }
    // ... other matches ...
}"#
        .to_string(),
        r#"Step 9: Update Dependencies
File: Cargo.toml

Ensure that llm_client and any other required crates are included.
If SummaryClient introduces new dependencies, add them accordingly.
Example:

[dependencies]
async-trait = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
llm_client = { path = "../path_to_llm_client" }
# ... other dependencies ..."#
            .to_string(),
        r#"Step 10: Add Unit Tests
File: tests/tool_code_summarization.rs (new file)

Write unit tests for SummaryClient:

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic::tool::output::ToolOutput;

    #[tokio::test]
    async fn test_code_summarization() {
        let llm_client = Arc::new(LLMBroker::new());
        let summary_client = SummaryClient::new(llm_client);

        let request = CodeSummarizationRequest {
            code: "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
            root_request_id: "test-root-id".to_string(),
        };

        let input = ToolInput::CodeSummarization(request);
        let output = summary_client.invoke(input).await.unwrap();

        if let ToolOutput::CodeSummarization(response) = output {
            assert!(!response.summary.is_empty());
            println!("Summary: {}", response.summary);
        } else {
            panic!("Expected CodeSummarization output");
        }
    }
}"#
        .to_string(),
    ]
    .iter()
    .map(|description| {
        PlanStep::new(
            Uuid::new_v4().to_string(),
            vec![], // this is key
            "title".to_owned(),
            description.to_owned(),
            UserContext::new(vec![], vec![], None, vec![]),
        )
    })
    .collect::<Vec<PlanStep>>();

    let mut plan_storage_path = default_index_dir();
    plan_storage_path = plan_storage_path.join("plans");

    // check if the plan_storage_path_exists
    if tokio::fs::metadata(&plan_storage_path).await.is_err() {
        tokio::fs::create_dir(&plan_storage_path)
            .await
            .expect("directory creation to not fail");
    }

    let plan_id = "test_plan".to_owned();
    plan_storage_path = plan_storage_path.join("test_plan");

    let plan = Plan::new(
        plan_id.to_owned(),
        plan_id.to_owned(),
        UserContext::new(vec![], vec![], None, vec![]),
        user_query,
        steps,
        plan_storage_path
            .to_str()
            .map(|plan_str| plan_str.to_owned())
            .expect("PathBuf to string conversion should work on each platform"),
    );

    let update_query = String::from("I'd actually want the tool name to be 'Repomap'");
    let new_context = String::from(
        r#"pub struct RepoMap {
    map_tokens: usize,
}

const REPOMAP_DEFAULT_TOKENS: usize = 1024;

impl RepoMap {
    pub fn new() -> Self {
        Self {
            map_tokens: REPOMAP_DEFAULT_TOKENS,
        }
    }

    pub fn with_map_tokens(mut self, map_tokens: usize) -> Self {
        self.map_tokens = map_tokens;
        self
    }

    pub async fn get_repo_map(&self, tag_index: &TagIndex) -> Result<String, RepoMapError> {
        let repomap = self.get_ranked_tags_map(self.map_tokens, tag_index).await?;

        if repomap.is_empty() {
            return Err(RepoMapError::TreeGenerationError(
                "No tree generated".to_string(),
            ));
        }

        println!("Repomap: {}k tokens", self.get_token_count(&repomap) / 1024);

        Ok(repomap)
    }
"#,
    );

    let request = PlanUpdateRequest::new(
        plan,
        new_context,
        0,
        update_query,
        request_id_str,
        editor_url,
    );

    let _updater = tool_broker.invoke(ToolInput::UpdatePlan(request)).await;

    // output / response boilerplate
}
