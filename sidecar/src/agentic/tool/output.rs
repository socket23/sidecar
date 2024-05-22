//! Contains the output of a tool which can be used by any of the callers

use super::{
    code_symbol::{
        correctness::CodeCorrectnessAction, followup::ClassSymbolFollowupResponse,
        important::CodeSymbolImportantResponse,
    },
    editor::apply::EditorApplyResponse,
    filtering::broker::{CodeToEditFilterResponse, CodeToEditSymbolResponse},
    grep::file::FindInFileResponse,
    lsp::{
        diagnostics::LSPDiagnosticsOutput,
        gotodefintion::GoToDefinitionResponse,
        gotoimplementations::GoToImplementationResponse,
        gotoreferences::GoToReferencesResponse,
        open_file::{OpenFileRequest, OpenFileResponse},
        quick_fix::{GetQuickFixResponse, LSPQuickFixInvocationResponse},
    },
    rerank::base::ReRankEntriesForBroker,
};

pub struct CodeToEditSnippet {
    start_line: i64,
    end_line: i64,
    thinking: String,
}

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

pub enum ToolOutput {
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
}

impl ToolOutput {
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

    pub fn class_symbols_to_followup(self) -> Option<ClassSymbolFollowupResponse> {
        match self {
            ToolOutput::ClassSymbolFollowupResponse(output) => Some(output),
            _ => None,
        }
    }
}
