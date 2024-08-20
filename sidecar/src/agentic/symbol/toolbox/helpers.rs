use crate::agentic::symbol::events::edit::SymbolToEdit;

pub type SymbolName = String;
pub type OriginalContent = String;
pub type UpdatedContent = String;
pub type Changes = Vec<(SymbolToEdit, OriginalContent, UpdatedContent)>;

#[derive(Debug, Clone)]
pub struct SymbolChanges {
    symbol_name: SymbolName,
    changes: Changes,
}

impl Default for SymbolChanges {
    fn default() -> Self {
        Self {
            symbol_name: String::new(),
            changes: Vec::new(),
        }
    }
}

impl SymbolChanges {
    pub fn new(symbol_name: SymbolName, changes: Changes) -> Self {
        Self {
            symbol_name,
            changes,
        }
    }

    pub fn add_change(
        &mut self,
        edit: SymbolToEdit,
        original_content: OriginalContent,
        updated_content: UpdatedContent,
    ) {
        self.changes.push((edit, original_content, updated_content));
    }

    pub fn symbol_name(&self) -> &SymbolName {
        &self.symbol_name
    }

    pub fn changes(&self) -> &Changes {
        &self.changes
    }
}

#[derive(Debug, Clone)]
pub struct SymbolChangeSet {
    changes: Vec<SymbolChanges>,
}

impl Default for SymbolChangeSet {
    fn default() -> Self {
        Self {
            changes: Vec::new(),
        }
    }
}

impl SymbolChangeSet {
    pub fn new(changes: Vec<SymbolChanges>) -> Self {
        Self { changes }
    }

    pub fn add_symbol_changes(&mut self, symbol_changes: SymbolChanges) {
        self.changes.push(symbol_changes);
    }

    pub fn get_changes_for_symbol(&self, symbol_name: &SymbolName) -> Option<&Changes> {
        self.changes
            .iter()
            .find(|sc| sc.symbol_name == *symbol_name)
            .map(|sc| &sc.changes)
    }

    pub fn changes(&self) -> &[SymbolChanges] {
        &self.changes
    }
}

use std::fmt;

impl fmt::Display for SymbolChangeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SymbolChangeSet {{")?;
        for (index, symbol_changes) in self.changes.iter().enumerate() {
            writeln!(f, "  Symbol {}: {}", index + 1, symbol_changes.symbol_name)?;
            for (change_index, (edit, original_content, _)) in
                symbol_changes.changes.iter().enumerate()
            {
                writeln!(f, "    Change {}: {:?}", change_index + 1, edit)?;
                writeln!(f, "      Original Content: {}", original_content)?;
            }
        }
        write!(f, "}}")
    }
}
