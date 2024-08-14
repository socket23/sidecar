use std::io;

use thiserror::Error;

use super::qa::{Answer, Question};

pub enum CommunicationInterface {
    Cli,
}

pub trait Communicator {
    fn ask_question(&self, question: &Question) -> Result<Answer, CommunicationError>;
}

#[derive(Debug, Error)]
pub enum CommunicationError {
    #[error("Input Error: {0}")]
    InputError(String),

    #[error("IO Error: {0}")]
    IoError(#[from] io::Error),
}
