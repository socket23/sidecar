//! Contains the output of a tool which can be used by any of the callers

use super::{
    code_symbol::important::CodeSymbolImportantResponse, lsp::diagnostics::LSPDiagnosticsOutput,
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
}
