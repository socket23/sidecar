use llm_client::clients::types::LLMClientMessage;

use super::{
    openai::OpenAILineEditPrompt,
    types::{
        InLineDocRequest, InLineEditPrompt, InLineEditRequest, InLineFixRequest,
        InLinePromptResponse,
    },
};

pub struct AnthropiLineEditPrompt {
    openai_line_edit: OpenAILineEditPrompt,
}

impl AnthropiLineEditPrompt {
    pub fn new() -> Self {
        Self {
            openai_line_edit: OpenAILineEditPrompt::new(),
        }
    }

    fn fix_inline_prompt_response(&self, response: InLinePromptResponse) -> InLinePromptResponse {
        match response {
            InLinePromptResponse::Completion(completion) => {
                InLinePromptResponse::Completion(completion)
            }
            InLinePromptResponse::Chat(chat_messages) => {
                let mut final_chat_messages = vec![];
                // the limitation we have here is that we have to concatenate all the consecutive
                // user and assistant messages together
                let mut previous_role = None;
                let mut pending_message: Option<LLMClientMessage> = None;
                for chat_message in chat_messages.into_iter() {
                    let role = chat_message.role().clone();
                    // if roles match, then we just append this to the our ongoing message
                    if previous_role == Some(role) {
                        if pending_message.is_some() {
                            pending_message = pending_message
                                .map(|pending_message| pending_message.concat(chat_message));
                        }
                    } else {
                        // if we have some previous message we should flush it
                        if let Some(pending_message_value) = pending_message {
                            final_chat_messages.push(pending_message_value);
                            pending_message = None;
                            previous_role = None;
                        }
                        // set the previous message and the role over here
                        previous_role = Some(chat_message.role().clone());
                        pending_message = Some(chat_message);
                    }
                }
                // if we still have some value remaining we push it to our chat messages
                if let Some(pending_message_value) = pending_message {
                    final_chat_messages.push(pending_message_value);
                }
                InLinePromptResponse::Chat(final_chat_messages)
            }
        }
    }
}

impl InLineEditPrompt for AnthropiLineEditPrompt {
    fn inline_edit(&self, request: InLineEditRequest) -> InLinePromptResponse {
        let inline_prompt_response = self.openai_line_edit.inline_edit(request);
        self.fix_inline_prompt_response(inline_prompt_response)
    }

    fn inline_fix(&self, request: InLineFixRequest) -> InLinePromptResponse {
        let inline_prompt_response = self.openai_line_edit.inline_fix(request);
        self.fix_inline_prompt_response(inline_prompt_response)
    }

    fn inline_doc(&self, request: InLineDocRequest) -> InLinePromptResponse {
        let inline_prompse_response = self.openai_line_edit.inline_doc(request);
        self.fix_inline_prompt_response(inline_prompse_response)
    }
}
