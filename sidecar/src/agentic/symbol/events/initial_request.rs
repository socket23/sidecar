#[derive(Debug, Clone, serde::Serialize)]
pub struct InitialRequestData {
    original_question: String,
}

impl InitialRequestData {
    pub fn new(original_question: String) -> Self {
        Self { original_question }
    }

    pub fn get_original_question(&self) -> &str {
        &self.original_question
    }
}
