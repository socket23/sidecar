use std::path::PathBuf;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    config::LLMBrokerConfiguration,
    provider::{LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};
use sidecar::agentic::symbol::identifier::LLMProperties;

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    let gpt4o_config = LLMProperties::new(
        LLMType::Gpt4OMini,
        LLMProvider::OpenAI,
        LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
            "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".to_owned(),
        )),
    );
    let llm_client = LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
        .await
        .expect("to work");
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let system_message = r#"You are a powerful code filtering engine. You must order the code snippets in the order in you want to edit them, and only those code snippets which should be edited.
- The code snippets will be provided to you in <code_snippet> section which will also have an id in the <id> section.
- You should select a code section for editing if and only if you want to make changes to that section.
- You are also given a list of extra symbols in <extra_symbols> which will be provided to you while making the changes, use this to be more sure about your reason for selection.
- Adding new functionality is a very valid reason to select a sub-section for editing.
- Editing or deleting some code is also a very valid reason for selecting a code section for editing.
- First think step by step on how you want to go about selecting the code snippets which are relevant to the user query in max 50 words.
- If you want to edit the code section with id 0 then you must output in the following format:
<code_to_edit_list>
<code_to_edit>
<id>
0
</id>
<reason_to_edit>
{your reason for editing}
</reason_to_edit>
</code_to_edit>
</code_to_edit_list>

- If you want to edit more code sections follow the similar pattern as described above and as an example again:
<code_to_edit_list>
<code_to_edit>
<id>
{id of the code snippet you are interested in}
</id>
<reason_to_edit>
{your reason for editing}
</reason_to_edit>
</code_to_edit>
{... more code sections here which you might want to select}
</code_to_edit_list>

- The <id> section should ONLY contain an id from the listed code snippets.


Here is an example contained in the <example> section.

<example>
<user_query>
We want to add a new method to add a new shipment made by the company.
</user_query>

<rerank_list>
<rerank_entry>
<id>
0
</id>
<content>
Code Location: company.rs
```rust
struct Company {
    name: String,
    shipments: usize,
    size: usize,
}
```
</content>
</rerank_entry>
<rerank_entry>
<id>
1
</id>
<content>
Code Location: company_metadata.rs
```rust
impl Compnay {
    fn name(&self) -> &str {
        &self.name
    }

    fn size(&self) -> usize {
        self.size
    }
}
</content>
</rerank_entry>
<rerank_entry>
<id>
2
</id>
<content>
Code Location: company_shipments.rs
```rust
impl Company {
    fn get_snipments(&self) -> usize {
        self.shipments
    }
}
```
</content>
</rerank_entry>
</rerank_list>

Your reply should be:

<thinking>
The company_shipment implementation block handles everything related to the shipments of the company, so we want to edit that.
</thinking>

<code_to_edit_list>
<code_to_edit>
<id>
2
</id>
<reason_to_edit>
The company_shipment.rs implementation block of Company contains all the relevant code for the shipment tracking of the Company, so that's what we want to edit.
</reason_to_edit>
<id>
</code_to_edit>
</code_to_edit_list>
</example>

This example is for reference. You must strictly follow the format show in the example when replying.
Please provide the list of symbols which you want to edit."#;
    let user_message = r#"<user_query>
Add a new variant called AskHuman to the ToolType enum
</user_query>

<extra_symbols>

</extra_symbols>

<rerank_entry>
<id>
0
</id>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/agentic/tool/type.rs:6-93
</file_path>
<content>
```
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
}
```
</content>
</rerank_entry>
<rerank_entry>
<id>
1
</id>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/agentic/tool/type.rs:95-158
</file_path>
<content>
```
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
        }
    }
}
```
</content>
</rerank_entry>"#;
    let start_instant = std::time::Instant::now();
    let response = llm_client
        .stream_completion(
            gpt4o_config.api_key().clone(),
            LLMClientCompletionRequest::new(
                gpt4o_config.llm().clone(),
                vec![
                    LLMClientMessage::system(system_message.to_owned()),
                    LLMClientMessage::user(user_message.to_owned()),
                ],
                0.2,
                None,
            ),
            gpt4o_config.provider().clone(),
            vec![].into_iter().collect(),
            sender,
        )
        .await;
    println!("elapsed_time: {}", start_instant.elapsed().as_millis());
    println!("{:?}", response);
}
