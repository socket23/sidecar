use std::sync::Arc;

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use async_trait::async_trait;
use quick_xml::de::from_str;

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReRankingCodeSnippetSymbolOutline {
    name: String,
    fs_file_path: String,
    content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReRankingSnippetsForCodeEditingRequest {
    outline_nodes: Vec<ReRankingCodeSnippetSymbolOutline>,
    // We should make these outline as well, we do not need all the verbose content
    // over here
    code_above: Option<String>,
    code_below: Option<String>,
    code_to_edit_selection: String,
    fs_file_path: String,
    user_query: String,
    llm_properties: LLMProperties,
    root_request_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_symbol")]
pub struct ReRankingCodeSymbol {
    name: String,
    #[serde(rename = "file_path")]
    fs_file_path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "reply")]
pub struct ReRankingSnippetsForCodeEditingResponse {
    thinking: String,
    code_symbol_outline_list: Vec<ReRankingCodeSymbol>,
}

impl ReRankingSnippetsForCodeEditingResponse {
    fn parse_response(response: &str) -> Result<Self, ToolError> {
        let parsed_response = from_str::<Self>(response);
        match parsed_response {
            Err(_e) => Err(ToolError::SerdeConversionFailed),
            Ok(parsed_response) => Ok(parsed_response),
        }
    }
}

pub struct ReRankingSnippetsForCodeEditingContext {
    llm_client: Arc<LLMBroker>,
    fail_over_llm: LLMProperties,
}

impl ReRankingSnippetsForCodeEditingContext {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            fail_over_llm,
        }
    }

    fn system_message(&self) -> String {
        format!(
            r"You are an expert software eningeer who never writes incorrect code. As a first step before making changes, you are tasked with collecting all the context you need for the definitions of various code symbols which will be necessary for making the code changes.
- You will be given the original user query in <user_query>
- You will be provided the code snippet you will be editing in <code_snippet_to_edit> section.
- The various definitions of the class (just the high level outline of it) will be given to you as a list in <code_symbol_outline_list>. When writing code you will reuse the methods from here to make the edits, so be very careful when selecting the symbol outlines you are interested in.
- Each code_symbol_outline entry is in the following format:
```
<code_symbol>
<name>
{{name of the code symbol over here}}
</name>
<content>
{{the outline content for the code symbol over here}}
</content>
</code_symbol>
```
- You have to decide which code symbols you will be using when doing the edits and select those code symbols.
Your reply should be in the following format:
<reply>
<thinking>
</thinking>
<code_symbol_outline_list>
<code_symbol>
<name>
</name>
<file_path>
</file_path>
</code_symbol>
... more code_symbol sections over here as per your requirement
</code_symbol_outline_list>
<reply>

Now we will show you some examples:
<user_query>

</user_query>
<code_snippet_to_edit>
</code_snippet_to_edit>
<code_symbol_outline_list>
<code_symbol>
<name>
</name>
<content>
</content>
</code_symbol>
<code_symbol>
<name>
</name>
<content>
</content>
</code_symbol>
<code_symbol>
<name>
</name>
<content>
</content>
</code_symbol>
<code_symbol>
<name>
</name>
<content>
</content>
</code_symbol>
</code_symbol_outline_list>

Your reply should be:
<reply>
<thinking>
</thinking>
<code_symbol_outline_list>
<code_symbol>
<name>
</name>
<file_path>
</file_path>
</code_symbol>
</code_symbol_outline_list>
</reply>"
        )
    }

    fn user_message(&self, user_context: &ReRankingSnippetsForCodeEditingRequest) -> String {
        let query = &user_context.user_query;
        let file_path = &user_context.fs_file_path;
        let code_interested = &user_context.code_to_edit_selection;
        let code_above = user_context
            .code_above
            .as_ref()
            .map(|code_above| {
                format!(
                    r#"<code_above>
{code_above}
</code_above>"#
                )
            })
            .unwrap_or("".to_owned());
        let code_below = user_context
            .code_below
            .as_ref()
            .map(|code_below| {
                format!(
                    r#"<code_below>
{code_below}
</code_below>"#
                )
            })
            .unwrap_or("".to_owned());
        let outline_nodes = user_context
            .outline_nodes
            .iter()
            .map(|outline_node| {
                let name = &outline_node.name;
                let file_path = &outline_node.fs_file_path;
                let content = &outline_node.content;
                format!(
                    r#"<code_symbol>
<name>
{name}
</name>
<file_path>
{file_path}
</file_path>
<content>
{content}
</content>
</code_symbol>"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"<user_query>
{query}
</user_query>

<file_path>
{file_path}
</file_path>

{code_above}
{code_below}
<code_in_selection>
{code_interested}
<?code_in_selection>

<code_symbol_outline_list>
{outline_nodes}
</code_symbol_outline_list>"#
        )
    }
}

#[async_trait]
impl Tool for ReRankingSnippetsForCodeEditingContext {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.reranking_code_snippets_for_editing_context()?;
        let root_request_id = context.root_request_id.to_owned();
        let llm_properties = context.llm_properties.clone();
        let system_message = LLMClientMessage::system(self.system_message());
        let user_message = LLMClientMessage::user(self.user_message(&context));
        let llm_request = LLMClientCompletionRequest::new(
            llm_properties.llm().clone(),
            vec![system_message, user_message],
            0.2,
            None,
        );
        let mut retries = 0;
        loop {
            if retries >= 4 {
                return Err(ToolError::RetriesExhausted);
            }
            let (llm, api_key, provider) = if retries % 2 == 1 {
                (
                    llm_properties.llm().clone(),
                    llm_properties.api_key().clone(),
                    llm_properties.provider().clone(),
                )
            } else {
                (
                    self.fail_over_llm.llm().clone(),
                    self.fail_over_llm.api_key().clone(),
                    self.fail_over_llm.provider().clone(),
                )
            };
            let cloned_message = llm_request.clone().set_llm(llm);
            let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
            let response = self
                .llm_client
                .stream_completion(
                    api_key,
                    cloned_message,
                    provider,
                    vec![
                        (
                            "event_type".to_owned(),
                            "reranking_code_snippets_for_editing_context".to_owned(),
                        ),
                        ("root_id".to_owned(), root_request_id.to_owned()),
                    ]
                    .into_iter()
                    .collect(),
                    sender,
                )
                .await;
            match response {
                Ok(response) => {
                    if let Ok(parsed_response) =
                        ReRankingSnippetsForCodeEditingResponse::parse_response(&response)
                    {
                        return Ok(ToolOutput::re_ranked_code_snippets_for_editing_context(
                            parsed_response,
                        ));
                    } else {
                        retries = retries + 1;
                        continue;
                    }
                }
                Err(_e) => {
                    retries = retries + 1;
                    continue;
                }
            }
        }
    }
}
