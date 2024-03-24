use super::types::{FillInMiddleFormatter, FillInMiddleRequest};
use either::Either;
use llm_client::clients::types::{
    LLMClientCompletionRequest, LLMClientCompletionStringRequest, LLMClientMessage,
};

pub struct ClaudeFillInMiddleFormatter;

impl ClaudeFillInMiddleFormatter {
    pub fn new() -> Self {
        Self
    }

    pub fn few_shot_messages(&self) -> Vec<LLMClientMessage> {
        vec![
            LLMClientMessage::user(
                r#"<prompt>
import random

def generate_random_number(min, max):
    return random.randint(min, max)

def print_random_number():
    random_number = generate_random_number(1, 10)
    <<CURSOR>>

print_random_number()
</prompt>"#
                    .to_owned(),
            ),
            LLMClientMessage::assistant(
                r#"<reply>
print(f"The random number is: {random_number}")
</reply>"#
                    .to_owned(),
            ),
            LLMClientMessage::user(
                r#"<prompt>
class Car:
    def __init__(self, make, model, year):
        self.make = make
        self.model = model
        self.year = year

    def get_car_details(self):
        <<CURSOR>>

    def get_year_details(self):
        return self.year

my_car = Car("Toyota", "Camry", 2022)

print(my_car.get_car_details())
</prompt>"#
                    .to_owned(),
            ),
            LLMClientMessage::assistant(
                r#"<reply>
return f"{self.year} {self.make} {self.model}"
</reply>"#
                    .to_owned(),
            ),
        ]
    }
}

impl FillInMiddleFormatter for ClaudeFillInMiddleFormatter {
    fn fill_in_middle(
        &self,
        request: FillInMiddleRequest,
    ) -> Either<LLMClientCompletionRequest, LLMClientCompletionStringRequest> {
        let system_prompt = r#"You are an intelligent code autocomplete model trained to generate code completions from the cursor position. Given a code snippet with a cursor position marked by <<CURSOR>>, your task is to generate the code that should appear after the cursor to complete the code logically.

To generate the code completion, follow these guidelines:
1. Analyze the code before and after the cursor position to understand the context and intent of the code.
2. If provided, utilize the relevant code snippets from other locations in the codebase to inform your completion.
3. Generate code that logically continues from the cursor position, maintaining the existing code structure and style.
4. Avoid introducing extra whitespace unless necessary for the code completion.
5. Output only the completed code, without any additional explanations or comments.

Remember, your goal is to provide the most appropriate and efficient code completion based on the given context and the location of the cursor. Use your programming knowledge and the provided examples to generate high-quality code completions that meet the requirements of the task."#;
        let prefix = request.prefix();
        let suffix = request.suffix();
        let fim_request = format!(
            r#"<prompt>
{prefix}<<CURSOR>>
{suffix}
</prompt>"#
        );
        let example_messages = self.few_shot_messages();
        let final_messages = vec![LLMClientMessage::system(system_prompt.to_owned())]
            .into_iter()
            .chain(example_messages)
            .chain(vec![LLMClientMessage::user(fim_request)])
            .collect::<Vec<_>>();
        let mut llm_request =
            LLMClientCompletionRequest::new(request.llm().clone(), final_messages, 0.1, None);
        if let Some(max_tokens) = request.completion_tokens() {
            llm_request = llm_request.set_max_tokens(max_tokens);
        }
        either::Left(llm_request)
    }
}
