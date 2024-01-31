use crate::clients::types::LLMClientMessage;

use super::types::{LLMFormatting, TokenizerConfig, TokenizerError};

pub struct CodeLLama70BInstructFormatting {
    tokenizer_config: TokenizerConfig,
}

impl CodeLLama70BInstructFormatting {
    pub fn new() -> Result<Self, TokenizerError> {
        let config = include_str!("tokenizer_config/codellama.json");
        let tokenizer_config = serde_json::from_str::<TokenizerConfig>(config)?;
        Ok(Self { tokenizer_config })
    }
}

impl LLMFormatting for CodeLLama70BInstructFormatting {
    fn to_prompt(&self, messages: Vec<LLMClientMessage>) -> String {
        // we want to convert the message to codellama format
        // persent here: https://huggingface.co/codellama/CodeLlama-70b-Instruct-hf/blob/main/tokenizer_config.json#L4
        // {% if messages[0]['role'] == 'system' %}
        // {% set user_index = 1 %}
        // {% else %}
        // {% set user_index = 0 %}
        // {% endif %}
        // {% for message in messages %}
        // {% if (message['role'] == 'user') != ((loop.index0 + user_index) % 2 == 0) %}
        // {{ raise_exception('Conversation roles must alternate user/assistant/user/assistant/...') }}
        // {% endif %}
        // {% if loop.index0 == 0 %}
        // {{ '<s>' }}
        // {% endif %}
        // {% set content = 'Source: ' + message['role'] + '\n\n ' + message['content'].strip() %}
        // {{ content + ' <step> ' }}
        // {% endfor %}
        // {{'Source: assistant\nDestination: user\n\n '}}
        println!("PRINTING MESSAGES");
        println!("{:?}", messages);
        println!("DONE");
        let formatted_message = messages
            .into_iter()
            .enumerate()
            .map(|(index, message)| {
                let content = message.content().trim();
                let role = match message.role() {
                    role if role.is_assistant() => "assistant",
                    role if role.is_user() => "user",
                    _ => "system",
                };
                let prefix = if index == 0 { "<s>" } else { "" };
                format!("{}Source: {}\n\n {} <step> ", prefix, role, content)
            })
            .collect::<Vec<_>>()
            .join("");
        let response = format!(
            "{} Source: assistant\nDestination: user\n\n ",
            formatted_message
        );
        // println!("{}", response);
        response
    }
}

#[cfg(test)]
mod tests {

    use crate::clients::types::LLMClientMessage;

    use super::CodeLLama70BInstructFormatting;
    use super::LLMFormatting;

    #[test]
    fn test_formatting_works() {
        let messages = vec![LLMClientMessage::user(
            "Write me a function to add 2 numbers in Rust".to_owned(),
        )];
        let codellama_formatting = CodeLLama70BInstructFormatting::new().unwrap();
        assert_eq!(
            codellama_formatting.to_prompt(messages),
            r#"<s>Source: system

           #### USER SELECTED CONTEXT ####
           ####
           Your job is to answer the user query.

           When referring to code, you must provide an example in a code block.

           - You are given the following project labels which are associated with the codebase:
           - rust
           - cargo


           Respect these rules at all times:
           - When asked for your name, you must respond with "Aide".
           - Follow the user's requirements carefully & to the letter.
           - Minimize any other prose.
           - Unless directed otherwise, the user is expecting for you to edit their selected code.
           - Link ALL paths AND code symbols (functions, methods, fields, classes, structs, types, variables, values, definitions, directories, etc) by embedding them in a markdown link, with the URL corresponding to the full path, and the anchor following the form `LX` or `LX-LY`, where X represents the starting line number, and Y represents the ending line number, if the reference is more than one line.
               - For example, to refer to lines 50 to 78 in a sentence, respond with something like: The compiler is initialized in [`src/foo.rs`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L50-L78)
               - For example, to refer to the `new` function on a struct, respond with something like: The [`new`](/Users/nareshr/github/codestory/sidecarsrc/bar.rs#L26-53) function initializes the struct
               - For example, to refer to the `foo` field on a struct and link a single line, respond with something like: The [`foo`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L138) field contains foos. Do not respond with something like [`foo`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L138-L138)
               - For example, to refer to a folder `foo`, respond with something like: The files can be found in [`foo`](/Users/nareshr/github/codestory/sidecarpath/to/foo/) folder
           - Do not print out line numbers directly, only in a link
           - Do not refer to more lines than necessary when creating a line range, be precise
           - Do NOT output bare symbols. ALL symbols must include a link
               - E.g. Do not simply write `Bar`, write [`Bar`](/Users/nareshr/github/codestory/sidecarsrc/bar.rs#L100-L105).
               - E.g. Do not simply write "Foos are functions that create `Foo` values out of thin air." Instead, write: "Foos are functions that create [`Foo`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L80-L120) values out of thin air."
           - Link all fields
               - E.g. Do not simply write: "It has one main field: `foo`." Instead, write: "It has one main field: [`foo`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L193)."
           - Do NOT link external urls not present in the context, do NOT link urls from the internet
           - Link all symbols, even when there are multiple in one sentence
               - E.g. Do not simply write: "Bars are [`Foo`]( that return a list filled with `Bar` variants." Instead, write: "Bars are functions that return a list filled with [`Bar`](/Users/nareshr/github/codestory/sidecarsrc/bar.rs#L38-L57) variants."
           - Code blocks MUST be displayed to the user using markdown
           - Code blocks MUST be displayed to the user using markdown and must NEVER include the line numbers
           - If you are going to not edit sections of the code, leave "// rest of code .." as the placeholder string
           - Do NOT write the line number in the codeblock
               - E.g. Do not write:
               ```rust
               1. // rest of code ..
               2. // rest of code ..
               ```
               Here the codeblock has line numbers 1 and 2, do not write the line numbers in the codeblock
           - You are given the code which the user has selected explicitly in the USER SELECTED CODE section
           - Pay special attention to the USER SELECTED CODE as these code snippets are specially selected by the user in their query <step> Source: user

            Write me a function to add 2 numbers in Rust <step> Source: assistant
           Destination: user


            "#
        );
    }
}
