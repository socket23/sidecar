use super::{
    base::ToolType,
    code_edit::{find::FindCodeSelectionInput, types::CodeEdit},
    code_symbol::important::{CodeSymbolImportantRequest, CodeSymbolImportantWideSearch},
    errors::ToolError,
    lsp::diagnostics::LSPDiagnosticsInput,
    rerank::base::ReRankEntriesForBroker,
};

pub enum ToolInput {
    CodeEditing(CodeEdit),
    LSPDiagnostics(LSPDiagnosticsInput),
    FindCodeSnippets(FindCodeSelectionInput),
    ReRank(ReRankEntriesForBroker),
    RequestImportantSymbols(CodeSymbolImportantRequest),
    RequestImportantSybmolsCodeWide(CodeSymbolImportantWideSearch),
}

impl ToolInput {
    pub fn tool_type(&self) -> ToolType {
        match self {
            ToolInput::CodeEditing(_) => ToolType::CodeEditing,
            ToolInput::LSPDiagnostics(_) => ToolType::LSPDiagnostics,
            ToolInput::FindCodeSnippets(_) => ToolType::FindCodeSnippets,
            ToolInput::ReRank(_) => ToolType::ReRank,
            ToolInput::RequestImportantSymbols(_) => ToolType::RequestImportantSymbols,
            ToolInput::RequestImportantSybmolsCodeWide(_) => ToolType::FindCodeSymbolsCodeBaseWide,
        }
    }

    pub fn is_code_edit(self) -> Result<CodeEdit, ToolError> {
        if let ToolInput::CodeEditing(code_edit) = self {
            Ok(code_edit)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn is_lsp_diagnostics(self) -> Result<LSPDiagnosticsInput, ToolError> {
        if let ToolInput::LSPDiagnostics(lsp_diagnostics) = self {
            Ok(lsp_diagnostics)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn is_code_find(self) -> Result<FindCodeSelectionInput, ToolError> {
        if let ToolInput::FindCodeSnippets(find_code_snippets) = self {
            Ok(find_code_snippets)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn is_rerank(self) -> Result<ReRankEntriesForBroker, ToolError> {
        if let ToolInput::ReRank(rerank) = self {
            Ok(rerank)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn code_symbol_search(
        self,
    ) -> Result<either::Either<CodeSymbolImportantRequest, CodeSymbolImportantWideSearch>, ToolError>
    {
        if let ToolInput::RequestImportantSymbols(request_code_symbol_important) = self {
            Ok(either::Either::Left(request_code_symbol_important))
        } else if let ToolInput::RequestImportantSybmolsCodeWide(request_code_symbol_important) =
            self
        {
            Ok(either::Either::Right(request_code_symbol_important))
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn code_symbol_important(self) -> Result<CodeSymbolImportantRequest, ToolError> {
        if let ToolInput::RequestImportantSymbols(request_code_symbol_important) = self {
            Ok(request_code_symbol_important)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }

    pub fn codebase_wide_important_symbols(
        self,
    ) -> Result<CodeSymbolImportantWideSearch, ToolError> {
        if let ToolInput::RequestImportantSybmolsCodeWide(request_code_symbol_important) = self {
            Ok(request_code_symbol_important)
        } else {
            Err(ToolError::WrongToolInput)
        }
    }
}
