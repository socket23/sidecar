use super::{
    error::CommunicationError,
    qa::{Answer, Question},
};

pub enum CommunicationInterface {
    Cli,
}

pub trait Communicator {
    fn ask_question(&self, question: &Question) -> Result<Answer, CommunicationError>;
}

struct HumanTool<T: Communicator> {
    communicator: T,
}

impl<T: Communicator> HumanTool<T> {
    fn new(communicator: T) -> Self {
        Self { communicator }
    }

    fn ask(&self, question: Question) -> Result<Answer, CommunicationError> {
        self.communicator.ask_question(&question)
    }
}
