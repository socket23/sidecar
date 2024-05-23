//! The different kind of events which the symbols can invoke and needs to work
//! on

use super::edit::SymbolToEditRequest;

#[derive(Debug, Clone)]
pub struct AskQuestionRequest {
    question: String,
}

impl AskQuestionRequest {
    pub fn new(question: String) -> Self {
        Self { question }
    }

    pub fn get_question(&self) -> &str {
        &self.question
    }
}

#[derive(Debug, Clone)]
pub enum SymbolEvent {
    InitialRequest,
    AskQuestion(AskQuestionRequest),
    UserFeedback,
    Delete,
    Edit(SymbolToEditRequest),
    Outline,
}
