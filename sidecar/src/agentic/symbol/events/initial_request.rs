#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolRequestHistoryItem {
    symbol: String,
    fs_file_path: String,
    request: String,
}

impl SymbolRequestHistoryItem {
    pub fn new(symbol: String, fs_file_path: String, request: String) -> Self {
        Self {
            symbol,
            fs_file_path,
            request,
        }
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol
    }

    pub fn fs_file_path(&self) -> &str {
        &self.fs_file_path
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InitialRequestData {
    original_question: String,
    plan_if_available: Option<String>,
    history: Vec<SymbolRequestHistoryItem>,
    /// We operate on the full symbol instead of the
    full_symbol_request: bool,
}

impl InitialRequestData {
    pub fn new(
        original_question: String,
        plan_if_available: Option<String>,
        history: Vec<SymbolRequestHistoryItem>,
        full_symbol_request: bool,
    ) -> Self {
        Self {
            original_question,
            plan_if_available,
            history,
            full_symbol_request,
        }
    }

    pub fn full_symbol_request(&self) -> bool {
        self.full_symbol_request
    }

    pub fn get_original_question(&self) -> &str {
        &self.original_question
    }

    pub fn get_plan(&self) -> Option<String> {
        self.plan_if_available.clone()
    }

    pub fn history(&self) -> &[SymbolRequestHistoryItem] {
        self.history.as_slice()
    }
}
