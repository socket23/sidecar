#[derive(Debug, Clone, serde::Serialize)]
pub struct InitialRequestData {
    original_question: String,
    plan_if_available: Option<String>,
}

impl InitialRequestData {
    pub fn new(original_question: String, plan_if_available: Option<String>) -> Self {
        Self {
            original_question,
            plan_if_available,
        }
    }

    pub fn get_original_question(&self) -> &str {
        &self.original_question
    }

    pub fn get_plan(&self) -> Option<String> {
        self.plan_if_available.clone()
    }
}
