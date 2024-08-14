use super::{
    error::CommunicationError,
    human::Communicator,
    qa::{Answer, Question},
};

use std::io::{self, Write};

pub struct CliCommunicator;

impl Communicator for CliCommunicator {
    fn ask_question(&self, question: &Question) -> Result<Answer, CommunicationError> {
        // Print the question
        println!("{}", question.text());

        // Flush stdout to ensure the question is displayed immediately
        io::stdout().flush()?;

        // Read user input
        let mut input = String::new();

        io::stdin().read_line(&mut input)?;

        // Trim whitespace and create an Answer
        let answer = Answer::new(input.trim().to_string());

        // Print the response
        println!("Your response: {}", answer.choice_id());

        Ok(answer)
    }
}
