use crate::{agentic::symbol::identifier::SymbolIdentifier, chunking::text_document::Range};

#[derive(Debug, Clone)]
pub struct SymbolToEdit {
    outline: bool,
    range: Range,
    fs_file_path: String,
    symbol_name: String,
    instructions: Vec<String>,
}

impl SymbolToEdit {
    pub fn new(
        symbol_name: String,
        range: Range,
        fs_file_path: String,
        instructions: Vec<String>,
        outline: bool,
    ) -> Self {
        Self {
            symbol_name,
            range,
            outline,
            fs_file_path,
            instructions,
        }
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn is_outline(&self) -> bool {
        self.outline
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub fn instructions(&self) -> &[String] {
        self.instructions.as_slice()
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }
}

#[derive(Debug, Clone)]
pub struct SymbolToEditRequest {
    symbols: Vec<SymbolToEdit>,
    symbol_identifier: SymbolIdentifier,
}

impl SymbolToEditRequest {
    pub fn new(symbols: Vec<SymbolToEdit>, identifier: SymbolIdentifier) -> Self {
        Self {
            symbol_identifier: identifier,
            symbols,
        }
    }

    pub fn symbols(&self) -> &[SymbolToEdit] {
        self.symbols.as_slice()
    }
}
