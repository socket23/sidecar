use async_trait::async_trait;
use futures::{stream, StreamExt};
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};
use quick_xml::de::from_str;
use std::{iter::once, sync::Arc, time::Instant};

use crate::{
    agentic::{
        symbol::{
            events::message_event::SymbolEventMessageProperties,
            identifier::LLMProperties,
            ui_event::{RelevantReference, UIEventWithID},
        },
        tool::{
            errors::ToolError, input::ToolInput, lsp::gotoreferences::AnchoredReference,
            output::ToolOutput, r#type::Tool,
        },
    },
    chunking::types::OutlineNode,
};

/// Represents a request for filtering references in the codebase.
#[derive(Debug, Clone)]
pub struct ReferenceFilterRequest {
    /// The instruction or query provided by the user.
    user_instruction: String,
    llm_properties: LLMProperties,
    /// The unique identifier for the root request.
    root_id: String,
    // we need ui_sender to fire events to ide
    message_properties: SymbolEventMessageProperties,
    anchored_references: Vec<AnchoredReference>,
}

impl ReferenceFilterRequest {
    pub fn new(
        user_instruction: String,
        llm_properties: LLMProperties,
        root_id: String,
        message_properties: SymbolEventMessageProperties,
        anchored_references: Vec<AnchoredReference>,
    ) -> Self {
        Self {
            user_instruction,
            llm_properties,
            root_id,
            message_properties,
            anchored_references,
        }
    }

    pub fn user_instruction(&self) -> &str {
        &self.user_instruction
    }

    pub fn llm_properties(&self) -> &LLMProperties {
        &self.llm_properties
    }

    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    pub fn message_properties(&self) -> &SymbolEventMessageProperties {
        &self.message_properties
    }

    pub fn anchored_references(&self) -> &[AnchoredReference] {
        &self.anchored_references
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename = "response")]
pub struct ReferenceFilterResponse {
    #[serde(rename = "reason")]
    pub reason: String,
    #[serde(rename = "change_required")]
    pub change_required: bool,
}

impl ReferenceFilterResponse {}

pub struct ReferenceFilterBroker {
    llm_client: Arc<LLMBroker>,
    _fail_over_llm: LLMProperties,
}

impl ReferenceFilterBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            _fail_over_llm: fail_over_llm,
        }
    }

    // consider variants: tiny, regular, in-depth
    pub fn system_message(&self) -> String {
        format!(
            r#"You are an expert software engineer who is pair programming with another developer.
- The developer you are helping has selected a symbol shown in <code_selected>
- They are considering a task, defined in <user_query>, to undertake against the symbol in <code_selected>
- First, summarise the change intended by <user_query> on <code_selected>, and write it to <change_description>
- Then, consider a dependency of the symbol in <code_selected>, provided in <reference>
- Identify which part of <reference> contains the dependent code
- Reason whether a change to the symbol in <reference> is absolutely necessary. Think precisely about the moving parts, do not guess, and be biased against making changes.
- Put your decision in <change_required> as either true or false
- Provide a single, high value sentence explaining whether a change is needed and why in <reason>
- Do not mistake the <user_query> for an instruction to <reference> - it's explicitly for <code_selected>

Your response must be in the following format:

<response>
<change_description>
summary of intended change on <code_selected> by <user_query>
</change_description>
<reason>
Your reason
</reason>
<change_required>
bool
</change_required>
</response>
"#
        )
    }

    pub fn user_message(
        &self,
        anchored_symbol_prompt: &str,
        user_query: &str,
        reference: &OutlineNode,
    ) -> String {
        format!(
            r#"<user_query>
{}
</user_query>

<code_selected>
{}
</code_selected>

<reference>
{}
</reference>"#,
            user_query,
            anchored_symbol_prompt,
            {
                let name = reference.name();
                let fs_file_path = reference.fs_file_path();
                let content = reference.content().content();

                format!("{} in {}\n{}", name, fs_file_path, content)
            }
        )
    }

    pub fn parse_response(response: &str) -> String {
        // println!("parse_response::response: {}", response);
        let answer = response
            .lines()
            .skip_while(|l| !l.contains("<reply>"))
            .skip(1)
            .take_while(|l| !l.contains("</reply>"))
            .collect::<Vec<&str>>()
            .join("\n");

        answer
    }

    pub fn few_shot_examples() -> Vec<LLMClientMessage> {
        let user_1 = LLMClientMessage::user(r#"<user_query>
add little_search
</user_query>

<code_selected>
/// Represents a request for filtering references in the codebase.
#[derive(Debug, Clone)]
pub struct ReferenceFilterRequest {
    /// The instruction or query provided by the user.
    user_instruction: String,
    llm_properties: LLMProperties,
    /// The unique identifier for the root request.
    root_id: String,
    // we need ui_sender to fire events to ide
    message_properties: SymbolEventMessageProperties,
    anchored_references: Vec<AnchoredReference>,
}
</code_selected>

<reference>
ReferenceFilterRequest in /Users/zi/codestory/testing/sidecar/sidecar/src/agentic/tool/ref_filter/ref_filter.rs
impl ReferenceFilterRequest {
    pub fn user_instruction(&self) -> &str {
        &self.user_instruction
    }

    pub fn llm_properties(&self) -> &LLMProperties {
        &self.llm_properties
    }

    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    pub fn message_properties(&self) -> &SymbolEventMessageProperties {
        &self.message_properties
    }

    pub fn anchored_references(&self) -> &[AnchoredReference] {
        &self.anchored_references
    }
}
</reference>"#.to_string());
        let system_1 = LLMClientMessage::system(r#"<reply>
<response>
<reason>
We do not need to change the code in the `ReferenceFilterRequest` struct, because the addition of `little_search` to the struct does not necessitate a getter method.
</reason>
<change_required>
false
</change_required>
</response>
</reply>"#.to_string());
        let user_2 = LLMClientMessage::user(
            r#"<user_query>
add little_search
</user_query>

<code_selected>
/// Represents a request for filtering references in the codebase.
#[derive(Debug, Clone)]
pub struct ReferenceFilterRequest {
    /// The instruction or query provided by the user.
    user_instruction: String,
    llm_properties: LLMProperties,
    /// The unique identifier for the root request.
    root_id: String,
    // we need ui_sender to fire events to ide
    message_properties: SymbolEventMessageProperties,
    anchored_references: Vec<AnchoredReference>,
}
</code_selected>

<reference>
ToolInput in /Users/zi/codestory/testing/sidecar/sidecar/src/agentic/tool/input.rs
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
}
</reference>"#
                .to_string(),
        );
        let system_2 = LLMClientMessage::system(r#"<reply>
<response>
<reason>
The ToolInput enum remains unchanged when adding little_search to ReferenceFilterRequest because it references the struct as a whole, not its internal fields.
</reason>
<change_required>
false
</change_required>
</response>
</reply>"#.to_string());

        vec![user_1, system_1, user_2, system_2]
    }
}

#[async_trait]
impl Tool for ReferenceFilterBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.filter_references_request()?;
        let llm_properties = context.llm_properties.clone();
        let root_request_id = context.root_id.to_owned();

        let system_message = LLMClientMessage::system(self.system_message());

        let user_query = context.user_instruction();

        let anchored_references = context.anchored_references().to_vec();

        println!(
            "anchored_references::count: {:?}",
            &anchored_references.len()
        );

        let relevant_references =
            stream::iter(anchored_references.into_iter().map(|anchored_reference| {
                // the user message contains:
                // 1. anchored symbol contents
                // 2. its reference's outline_node
                let user_message = self.user_message(
                    &anchored_reference.anchored_symbol().content(), // content of the anchored symbol
                    user_query,
                    &anchored_reference.ref_outline_node(), // outline node of the reference
                );

                let llm_messages = once(system_message.clone())
                    // .chain(Self::few_shot_examples())
                    .chain(once(LLMClientMessage::user(user_message)))
                    .collect::<Vec<_>>();

                let fs_file_path_for_reference = anchored_reference
                    .reference_location()
                    .fs_file_path()
                    .to_owned();
                let ref_symbol_name = anchored_reference.ref_outline_node().name().to_owned();
                (
                    LLMClientCompletionRequest::new(
                        llm_properties.llm().clone(),
                        llm_messages,
                        0.2,
                        None,
                    ),
                    self.llm_client.clone(),
                    llm_properties.clone(),
                    root_request_id.to_owned(),
                    context.clone(),
                    fs_file_path_for_reference,
                    ref_symbol_name,
                )
            }))
            .map(
                |(
                    request,
                    llm_client,
                    llm_properties,
                    root_request_id,
                    context,
                    fs_file_path_reference,
                    ref_symbol_name_reference,
                )| async move {
                    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
                    let start = Instant::now();

                    let response = llm_client
                        .stream_completion(
                            llm_properties.api_key().clone(),
                            request,
                            llm_properties.provider().clone(),
                            vec![
                                ("event_type".to_owned(), "filter_references".to_owned()),
                                ("root_id".to_owned(), root_request_id.to_owned()),
                            ]
                            .into_iter()
                            .collect(),
                            sender,
                        )
                        .await;
                    println!(
                        "reference_check::stream_completion::elapsed: {:?}",
                        start.elapsed()
                    );

                    println!("{:?}", &response);

                    let parsed_response = match response {
                        Ok(response) => from_str::<ReferenceFilterResponse>(&&response).ok(),
                        Err(_) => None,
                    };

                    println!("parsed_response:\n{:?}", parsed_response);

                    if let Some(parsed_response) = parsed_response {
                        if parsed_response.change_required {
                            let ui_sender = context.message_properties().ui_sender();
                            let _ = ui_sender.send(UIEventWithID::relevant_reference(
                                root_request_id.to_owned(),
                                &fs_file_path_reference,
                                &ref_symbol_name_reference,
                                &parsed_response.reason,
                            ));

                            Some(RelevantReference::new(
                                &fs_file_path_reference,
                                &ref_symbol_name_reference,
                                &parsed_response.reason,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                },
            )
            .buffer_unordered(200)
            .filter_map(|result| async move { result })
            .collect::<Vec<_>>()
            .await;

        Ok(ToolOutput::ReferencesFilter(relevant_references))
    }
}
