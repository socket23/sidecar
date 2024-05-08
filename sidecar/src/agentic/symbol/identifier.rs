//! Identifier here represents how the code will look like if we have metadata and the
//! location for it
//! We can also use the tools along with this symbol to traverse the code graph

use crate::chunking::text_document::Range;

#[derive(Debug, Eq, PartialEq)]
pub struct Snippet {
    range: Range,
    symbol_name: String,
    fs_file_path: String,
}

impl Snippet {
    pub fn new(symbol_name: String, range: Range, fs_file_path: String) -> Self {
        Self {
            symbol_name,
            range,
            fs_file_path,
        }
    }

    pub fn file_path(&self) -> &str {
        &self.fs_file_path
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SymbolIdentifier {
    symbol_name: String,
    fs_file_path: Option<String>,
}

impl SymbolIdentifier {
    pub fn new_symbol(symbol_name: &str) -> Self {
        Self {
            symbol_name: symbol_name.to_owned(),
            fs_file_path: None,
        }
    }

    pub fn with_file_path(symbol_name: &str, fs_file_path: &str) -> Self {
        Self {
            symbol_name: symbol_name.to_owned(),
            fs_file_path: Some(fs_file_path.to_owned()),
        }
    }
}

#[derive(Debug)]
pub struct MechaCodeSymbolThinking {
    symbol_name: String,
    steps: Vec<String>,
    is_new: bool,
    file_path: String,
    snippet: Option<Snippet>,
    implementations: Vec<Snippet>,
}

impl MechaCodeSymbolThinking {
    pub fn new(
        symbol_name: String,
        steps: Vec<String>,
        is_new: bool,
        file_path: String,
        snippet: Option<Snippet>,
        implementations: Vec<Snippet>,
    ) -> Self {
        Self {
            symbol_name,
            steps,
            is_new,
            file_path,
            snippet,
            implementations,
        }
    }

    pub fn is_new(&self) -> bool {
        self.is_new
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub fn to_symbol_identifier(&self) -> SymbolIdentifier {
        if self.is_new {
            SymbolIdentifier::new_symbol(&self.symbol_name)
        } else {
            SymbolIdentifier::with_file_path(&self.symbol_name, &self.file_path)
        }
    }

    pub fn set_snippet(&mut self, snippet: Snippet) {
        self.snippet = Some(snippet);
    }

    pub fn get_snippet(&self) -> Option<&Snippet> {
        self.snippet.as_ref()
    }

    pub fn add_step(&mut self, step: &str) {
        self.steps.push(step.to_owned());
    }

    pub fn fs_file_path(&self) -> &str {
        &self.file_path
    }
}
