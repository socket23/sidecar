use crate::{agent::llm_funcs, chunking::text_document::DocumentSymbol};

use super::{prompts, types::InLineAgentAction};

/// This is where we generate the messages for the documentation flow, we want
/// to craft the prompt

impl InLineAgentAction {
    pub fn prompt_for_documentation_generation(
        &self,
        document_symbols: Vec<DocumentSymbol>,
        language: &str,
        file_path: &str,
        query: &str,
    ) -> Vec<llm_funcs::llm::Messages> {
        document_symbols
            .into_iter()
            .map(|document_symbol| {
                let system_message = llm_funcs::llm::Message::system(
                    &prompts::documentation_system_prompt(language, document_symbol.kind.is_some()),
                );
                // Here we want to generate the context for the prompt
                let code_selection_prompt = llm_funcs::llm::Message::user(
                    &self.document_symbol_prompt(&document_symbol, language, file_path),
                );
                let user_prompt = format!(
                    "{} {}",
                    self.document_symbol_metadata(&document_symbol, language,),
                    query,
                );
                let metadata_prompt = llm_funcs::llm::Message::user(&user_prompt);
                llm_funcs::llm::Messages {
                    messages: vec![system_message, code_selection_prompt, metadata_prompt],
                }
            })
            .collect::<Vec<_>>()
    }

    fn document_symbol_prompt(
        &self,
        document_symbol: &DocumentSymbol,
        language: &str,
        file_path: &str,
    ) -> String {
        let code = &document_symbol.code;
        let prompt_string = format!(
            r"#I have the following code in the selection:
```{language}
// FILEPATH: {file_path}
{code}
```
#"
        );
        prompt_string
    }

    fn document_symbol_metadata(&self, document_symbol: &DocumentSymbol, language: &str) -> String {
        let comment_type = match language {
            "typescript" | "typescriptreact" => match document_symbol.kind {
                Some(_) => "a TSDoc comment".to_owned(),
                None => "TSDoc comment".to_owned(),
            },
            "javascript" | "javascriptreact" => match document_symbol.kind {
                Some(_) => "a JSDoc comment".to_owned(),
                None => "JSDoc comment".to_owned(),
            },
            "python" => "docstring".to_owned(),
            "rust" => "Rustdoc comment".to_owned(),
            _ => "documentation comment".to_owned(),
        };

        // Now we want to generate the document symbol metadata properly
        match &document_symbol.name {
            Some(name) => {
                format!("Please add {comment_type} for {name}.")
            }
            None => {
                format!("Please add {comment_type} for the selection.")
            }
        }
    }
}
