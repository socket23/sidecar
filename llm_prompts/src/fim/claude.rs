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
}

impl FillInMiddleFormatter for ClaudeFillInMiddleFormatter {
    fn fill_in_middle(
        &self,
        request: FillInMiddleRequest,
    ) -> Either<LLMClientCompletionRequest, LLMClientCompletionStringRequest> {
        let system_prompt = r#"You are an intelligent code autocomplete model. You are trained to autocomplete code from the cursor position.
You have to only type out the code after the cursor position.

Follow these rules at all times:
- You will also be given the prefix and the suffix from the cursor position, use that to generate the code after the cursor position.
- You are also given code snippets from other locations in the codebase which are relevant, use them for the completion as well.
- Do not introduce extra white spaces if the code completion does not require it at the cursor location
- You have to only complete the code, do not output anything else adhering to the output format.

The cursor position is indicated by <<CURSOR>>

As an example if the prompt you are given is:

<prompt>
def multiply(a, b):
    return a * b

def add(a, b):
    <<CURSOR>>

def subtract(a, b):
    return a - b
</prompt>

You should reply:
return a + b

Another example:
<prompt>
def roll_dice():
    return <<CURSOR>>
# --------^ Type a space here and it should autocomplete to random.randint(0, 6)!
</prompt>

You should reply:
random.randint(0, 6)"#;
        let prefix = request.prefix();
        let suffix = request.suffix();
        let fim_request = format!(
            r#"<prompt>
{prefix}<<CURSOR>>
{suffix}
</prompt>"#
        );
        let mut llm_request = LLMClientCompletionRequest::new(
            request.llm().clone(),
            vec![
                LLMClientMessage::system(system_prompt.to_owned()),
                LLMClientMessage::user(fim_request),
            ],
            0.1,
            None,
        );
        if let Some(max_tokens) = request.completion_tokens() {
            llm_request = llm_request.set_max_tokens(max_tokens);
        }
        either::Left(llm_request)
    }
}
