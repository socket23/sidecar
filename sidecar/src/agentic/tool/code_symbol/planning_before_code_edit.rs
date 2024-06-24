//! We allow for another round of COT to happen here before we start editing
//! This is to show the agent the code symbols it has gathered and come up with
//! an even better plan after the initial fetch

use async_trait::async_trait;
use quick_xml::de::from_str;
use std::{collections::HashMap, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{base::Tool, errors::ToolError, input::ToolInput, output::ToolOutput},
};

fn escape_xml(s: String) -> String {
    s.replace("\"", "&quot;")
        .replace("'", "&apos;")
        .replace(">", "&gt;")
        .replace("<", "&lt;")
        .replace("&", "&amp;")
}

fn dirty_unescape_fix(s: String) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

fn unescape_xml(s: String) -> String {
    quick_xml::escape::unescape(&s)
        .map(|output| output.to_string())
        .unwrap_or(dirty_unescape_fix(s))
        .to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanningBeforeCodeEditRequest {
    user_query: String,
    files_with_content: HashMap<String, String>,
    original_plan: String,
    llm_properties: LLMProperties,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "symbol")]
pub struct CodeEditingSymbolPlan {
    symbol_name: String,
    file_path: String,
    plan: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "symbol_list")]
pub struct PlanningBeforeCodeEditResponse {
    #[serde(rename = "$value")]
    final_plan_list: Vec<CodeEditingSymbolPlan>,
}

impl PlanningBeforeCodeEditResponse {
    fn unescape_plan_string(self) -> Self {
        let final_plan_list = self
            .final_plan_list
            .into_iter()
            .map(|plan_item| {
                let symbol_name = plan_item.symbol_name;
                let file_path = plan_item.file_path;
                let plan = plan_item
                    .plan
                    .lines()
                    .map(|line| unescape_xml(line.to_owned()))
                    .collect::<Vec<_>>()
                    .join("\n");
                CodeEditingSymbolPlan {
                    symbol_name,
                    file_path,
                    plan,
                }
            })
            .collect::<Vec<_>>();
        Self { final_plan_list }
    }

    fn parse_response(response: &str) -> Result<Self, ToolError> {
        let tags_to_check = vec![
            "<reply>",
            "</reply>",
            "<thinking>",
            "</thinking>",
            "<symbol_list>",
            "</symbol_list>",
        ];
        if tags_to_check.into_iter().any(|tag| !response.contains(tag)) {
            return Err(ToolError::MissingXMLTags);
        }
        // otherwise its correct and we need to grab the content between the <code_symbol> tags
        let lines = response
            .lines()
            .skip_while(|line| !line.contains("<symbol_list>"))
            .skip(1)
            .take_while(|line| !line.contains("</symbol_list>"))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = format!(
            r#"<symbol_list>
{lines}
</symbol_list>"#
        );

        let mut final_lines = vec![];
        let mut is_inside = false;
        for line in lines.lines() {
            if line == "<plan>" {
                is_inside = true;
                final_lines.push(line.to_owned());
            } else if line == "</plan>" {
                is_inside = false;
                final_lines.push(line.to_owned());
            }
            if is_inside {
                final_lines.push(escape_xml(line.to_owned()));
            } else {
                final_lines.push(line.to_owned());
            }
        }

        let parsed_response = from_str::<PlanningBeforeCodeEditResponse>(&final_lines.join("\n"));
        match parsed_response {
            Err(_e) => Err(ToolError::SerdeConversionFailed),
            Ok(parsed_list) => Ok(parsed_list.unescape_plan_string()),
        }
    }
}

pub struct PlanningBeforeCodeEdit {
    llm_client: Arc<LLMBroker>,
}

impl PlanningBeforeCodeEdit {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    fn system_message(&self) -> String {
        r#"You are an expert software engineer who has to come up with a plan to help with the user query. A junior engineer has already taken a pass at identifying the important code symbols in the codebase and a plan to tackle the problem. Your job is to take that plan, and analyse the code and correct any mistakes in the plan and make it more informative. You never make mistakes when coming up with the plan.
- The user query will be provided in <user_query> section of the message.
- We are working at the level of code symbols, which implies that when coming up with a plan to help, you should only select the symbols which are present in the code. Code Symbols can be functions, classes, enums, types etc.
as an example:
```rust
struct Something {{
    // rest of the code..
}}
```
is a code symbol since it represents a struct in rust, similarly
```py
def something():
    pass
```
is a code symbol since it represents a function in python.
- The original plan will be provided to you in the <original_plan> section of the message.
- We will show you the full file content where the selected code symbols are present, this is present in the <files_in_selection> section. You should use this to analyse if the plan has covered all the code symbols which need editing or changes.
- Deeply analyse the provided files in <files_in_selection> along with the <original_plan> and the <user_query> and come up with a detailed plan of what changes needs to made and the order in which the changes need to happen. If you think any code symbol is missing or not present in the selection, you should tell us about it in <extra_data> section of your answer.
- First let's think step-by-step on how to reply to the user query and then reply to the user query.
- The output should be strictly in the following format:
<reply>
<thinking>
{{your thoughts here on how to go about solving the problem and analysing the original plan which was created}}
</thinking>
<symbol_list>
<symbol>
<name>
{{name of the symbol you want to change}}
</name>
<file_path>
{{file path of the symbol where its present, this should be the absolute path as given to you in the original query}}
</file_path>
<plan>
{{your modified plan for this symbol}}
</plan>
</symbol>
{{more symbols here following the same format as above}}
</symbol_list>
</reply>"#.to_owned()
    }

    fn user_query(&self, request: PlanningBeforeCodeEditRequest) -> String {
        let user_query = request.user_query;
        let original_plan = request.original_plan;
        let files_with_content = request
            .files_with_content
            .into_iter()
            .map(|(file_path, content)| {
                format!(
                    r#"<file_content>
<file_path>
{file_path}
</file_path>
<content>
{content}
</content>
</file_content>"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"<user_query>
{user_query}
</user_query>

<files_in_selection>
{files_with_content}
</files_in_selection>

<original_plan>
{original_plan}
</original_plan>"#
        )
    }
}

#[async_trait]
impl Tool for PlanningBeforeCodeEdit {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.plan_before_code_editing()?;
        let llm_properties = context.llm_properties.clone();
        let system_message = LLMClientMessage::system(self.system_message());
        let user_message = LLMClientMessage::user(self.user_query(context));
        let message_request = LLMClientCompletionRequest::new(
            llm_properties.llm().clone(),
            vec![system_message, user_message],
            0.2,
            None,
        );
        let mut retries = 0;
        loop {
            if retries > 4 {
                return Err(ToolError::MissingXMLTags);
            }
            let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
            let response = self
                .llm_client
                .stream_completion(
                    llm_properties.api_key().clone(),
                    message_request.clone(),
                    llm_properties.provider().clone(),
                    vec![("event_type".to_owned(), "plan_before_code_edit".to_owned())]
                        .into_iter()
                        .collect(),
                    sender,
                )
                .await;
            if let Ok(response) = response {
                if let Ok(parsed_response) =
                    PlanningBeforeCodeEditResponse::parse_response(&response)
                {
                    // Now parse the response over here in the format we want it to be in
                    // we need to take care of the xml tags like ususal over here.. sigh
                    return Ok(ToolOutput::planning_before_code_editing(parsed_response));
                } else {
                    retries = retries + 1;
                }
            } else {
                retries = retries + 1;
            }
        }
    }
}
