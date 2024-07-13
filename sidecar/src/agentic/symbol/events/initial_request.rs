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
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InitialRequestData {
    original_question: String,
    plan_if_available: Option<String>,
    history: Vec<SymbolRequestHistoryItem>,
}

impl InitialRequestData {
    pub fn new(
        original_question: String,
        plan_if_available: Option<String>,
        history: Vec<SymbolRequestHistoryItem>,
    ) -> Self {
        Self {
            original_question,
            plan_if_available,
            history,
        }
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
