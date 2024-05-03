use llm_client::clients::types::{LLMClientCompletionRequest, LLMClientMessage};

use crate::agentic::tool::code_edit::types::CodeEdit;

use super::broker::CodeEditPromptFormatters;

pub struct AnthropicCodeEditFromatter {}

impl AnthropicCodeEditFromatter {
    pub fn new() -> Self {
        Self {}
    }

    fn system_message(&self, language: &str, file_path: &str) -> String {
        format!(
            r#"You are an expert software engineer who writes the most high quality code without making any mistakes.
Follow the user's requirements carefully and to the letter.
- The user instructions are present in <user_instruction> tag.
- Modify the code or create new code.
- The code present above the section you have to edit will be given in <code_above> section.
- The code present below the section you have to edit will be given in <code_below> section.
- The code you have to rewrite will be given to you in <code_to_edit> section.
- User the additional context provided to you in <extra_data> section to understand the functions avaialable on different types of variables, it might have additional context provided by the user, use them as required.
- The code you have to edit is in {file_path}
- Output the edited code in a single code block.
- Each code block starts with ```{language}.
- You must always answer in {language} code."#
        )
    }

    fn above_selection(&self, above_selection: Option<&str>) -> Option<String> {
        if let Some(above_selection) = above_selection {
            Some(format!(
                r#"I have the following code above:
<code_above>
{above_selection}
</code_above>"#
            ))
        } else {
            None
        }
    }

    fn below_selection(&self, below_selection: Option<&str>) -> Option<String> {
        if let Some(below_selection) = below_selection {
            Some(format!(
                r#"I have the following code below:
<code_below>
{below_selection}
</code_below>"#
            ))
        } else {
            None
        }
    }

    fn selection_to_edit(&self, selection_to_edit: &str) -> String {
        format!(
            r#"I have the following code in selection to edit:
<code_to_edit>
{selection_to_edit}
</code_to_edit>"#
        )
    }

    fn extra_data(&self, extra_data: &str) -> String {
        format!(
            r#"This is the extra data which you can use:
<extra_data>
{extra_data}
</extra_data>"#
        )
    }
}

impl CodeEditPromptFormatters for AnthropicCodeEditFromatter {
    fn format_prompt(&self, context: &CodeEdit) -> LLMClientCompletionRequest {
        let extra_data = self.extra_data(context.extra_content());
        let above = self.above_selection(context.above_context());
        let below = self.below_selection(context.below_context());
        let in_range = self.selection_to_edit(context.code_to_edit());
        let language = context.language();
        let fs_file_path = context.fs_file_path();
        let system_message = self.system_message(language, fs_file_path);
        let mut messages = vec![];

        // add the system message
        messages.push(LLMClientMessage::system(system_message));

        let mut user_message = "".to_owned();
        user_message = user_message + &extra_data + "\n";
        if let Some(above) = above {
            user_message = user_message + &above + "\n";
        }
        if let Some(below) = below {
            user_message = user_message + &below + "\n";
        }
        user_message = user_message + &in_range + "\n";

        // Now we add the instruction from the user
        let user_instruction = context.instruction();
        user_message = user_message
            + &format!(
                r#"Only edit the code in <code_to_edit> section, my instructions are:
<user_instruction>
{user_instruction}
</user_instruction>"#
            );

        // Now add the user message to the messages
        messages.push(LLMClientMessage::user(user_message));
        // we use 0.2 temperature so the model can imagine ✨
        LLMClientCompletionRequest::new(context.model().clone(), messages, 0.2, None)
    }
}
