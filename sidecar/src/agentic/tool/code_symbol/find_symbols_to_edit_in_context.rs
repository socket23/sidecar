use async_trait::async_trait;
use serde_xml_rs::from_str;
use std::sync::Arc;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct FindSymbolsToEditInContextRequest {
    context: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FindSymbolsToEditInContextSymbolList {
    #[serde(rename = "symbol")]
    symbols: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FindSymbolsToEditInContextResponse {
    thinking: String,
    symbol_list: FindSymbolsToEditInContextSymbolList,
}

/// Find symbols to edit in a context
pub struct FindSymbolsToEditInContext {
    llm_client: Arc<LLMBroker>,
    gemini_llm_properties: LLMProperties,
}

impl FindSymbolsToEditInContext {
    pub fn new(llm_client: Arc<LLMBroker>, gemini_llm_properties: LLMProperties) -> Self {
        Self {
            llm_client,
            gemini_llm_properties,
        }
    }

    fn system_message(&self) -> String {
        r#"You are an expert software engineer who is an expert at recognising the code symbols which are present in the user provided message.
- Code Symbols here refers to the class or struct or enum or type which are defined globally.
- If the code symbol is referring to a function in the struct, for example: in rust `SomeClass::function` we want to only get back `SomeClass`, in case of python or typescript if we have `SomeClass.function` we should only get back `SomeClass`
- Make sure to include all the code symbols which are present in the provided user context.
- Only include the symbols which require editing, adding or removing

Your reply should be in the following format:
<reply>
<thinking>
</thinking>
<symbol_list>
<symbol>
{name of the symbol}
</symbol>
... { more symbols over here}
</symbol_list>
</reply>"#.to_owned()
    }

    fn user_message(&self, request: FindSymbolsToEditInContextRequest) -> String {
        let context = request.context;
        format!(
            r#"<user_query>
{context}
</user_query>"#
        )
    }
}

#[async_trait]
impl Tool for FindSymbolsToEditInContext {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.find_symbols_to_edit_in_context()?;
        let system_message = LLMClientMessage::system(self.system_message());
        let user_message = LLMClientMessage::user(self.user_message(context));
        let message_request = LLMClientCompletionRequest::new(
            self.gemini_llm_properties.llm().clone(),
            vec![system_message, user_message],
            0.2,
            None,
        );
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let response = self
            .llm_client
            .stream_completion(
                self.gemini_llm_properties.api_key().clone(),
                message_request,
                self.gemini_llm_properties.provider().clone(),
                vec![(
                    "event_type".to_owned(),
                    "find_symbols_to_edit_in_context".to_owned(),
                )]
                .into_iter()
                .collect(),
                sender,
            )
            .await
            .map_err(|e| ToolError::LLMClientError(e))?;
        let parsed_response = from_str::<FindSymbolsToEditInContextResponse>(&response)
            .map_err(|_e| ToolError::SerdeConversionFailed)?;
        Ok(ToolOutput::find_symbols_to_edit_in_context(parsed_response))
    }
}

#[cfg(test)]
mod tests {
    use serde_xml_rs::from_str;

    use super::FindSymbolsToEditInContextResponse;

    #[test]
    fn test_parsing_response_works() {
        let response = r#"<reply>
<thinking>
The user query suggests that the code symbols `LLMProvider` and `GrokClient` are missing or not defined in the current scope.
</thinking>
<symbol_list>
<symbol>
LLMProvider
</symbol>
<symbol>
GrokClient
</symbol>
</symbol_list>
</reply>"#;

        let parsed_output = from_str::<FindSymbolsToEditInContextResponse>(&response);
        assert!(parsed_output.is_ok());
    }
}
