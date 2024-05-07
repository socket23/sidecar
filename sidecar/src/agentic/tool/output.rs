//! Contains the output of a tool which can be used by any of the callers

use super::{
    code_symbol::important::CodeSymbolImportantResponse,
    grep::file::FindInFileResponse,
    lsp::{
        diagnostics::LSPDiagnosticsOutput,
        gotodefintion::GoToDefinitionResponse,
        gotoimplementations::GoToImplementationResponse,
        open_file::{OpenFileRequest, OpenFileResponse},
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
    FileOpen(OpenFileResponse),
    GrepSingleFile(FindInFileResponse),
    GoToImplementation(GoToImplementationResponse),
}

impl ToolOutput {
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

    pub fn go_to_definition(go_to_definition: GoToDefinitionResponse) -> Self {
        ToolOutput::GoToDefinition(go_to_definition)
    }

    pub fn file_open(file_open: OpenFileResponse) -> Self {
        ToolOutput::FileOpen(file_open)
    }

    pub fn go_to_implementation(go_to_implementation: GoToImplementationResponse) -> Self {
        ToolOutput::GoToImplementation(go_to_implementation)
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
}
