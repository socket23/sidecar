use thiserror::Error;

pub struct Question {
    text: String,
    choices: Vec<Choice>,
}

pub struct Choice {
    id: String,
    text: String,
}

pub struct Answer {
    question_id: String,
    choice_id: String,
}

enum CommunicationInterface {
    Cli,
}

trait Communicator {
    fn ask_question(&self, question: &Question) -> Result<Answer, CommunicationError>;
}

#[derive(Debug, Error)]
enum CommunicationError {
    #[error("Input Error: {0}")]
    InputError(String),
}
