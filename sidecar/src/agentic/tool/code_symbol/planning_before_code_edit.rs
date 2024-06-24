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
    tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
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

impl PlanningBeforeCodeEditRequest {
    pub fn new(
        user_query: String,
        files_with_content: HashMap<String, String>,
        original_plan: String,
        llm_properties: LLMProperties,
    ) -> Self {
        Self {
            user_query,
            files_with_content,
            original_plan,
            llm_properties,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "symbol")]
pub struct CodeEditingSymbolPlan {
    #[serde(rename = "name")]
    symbol_name: String,
    file_path: String,
    plan: String,
}

impl CodeEditingSymbolPlan {
    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn plan(&self) -> &str {
        &self.plan
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "symbol_list")]
pub struct PlanningBeforeCodeEditResponse {
    #[serde(rename = "$value")]
    final_plan_list: Vec<CodeEditingSymbolPlan>,
}

impl PlanningBeforeCodeEditResponse {
    pub fn final_plan_list(self) -> Vec<CodeEditingSymbolPlan> {
        self.final_plan_list
    }

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
- Deeply analyse the provided files in <files_in_selection> along with the <original_plan> and the <user_query> and come up with a detailed plan of what changes needs to made and the order in which the changes need to happen.
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
                    println!("tool::planning_before_code_edit::parsed::success");
                    // Now parse the response over here in the format we want it to be in
                    // we need to take care of the xml tags like ususal over here.. sigh
                    return Ok(ToolOutput::planning_before_code_editing(parsed_response));
                } else {
                    println!("tool::planning_before_code_edit::parsed::error");
                    retries = retries + 1;
                }
            } else {
                println!("tool::planning_before_code_edit::response::error");
                retries = retries + 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlanningBeforeCodeEditResponse;

    #[test]
    fn test_parsing_output_works() {
        let response = r#"
        <reply>
        <thinking>
        After analyzing the code and the original plan, I believe the plan is mostly correct but needs some adjustments and additional details:
        
        1. The `eval_expr` function is indeed the right place to start, but we need to be more specific about how to check if the expression is safe to evaluate.
        
        2. The `parse_expr` function doesn't need to be modified as suggested. It already uses `stringify_expr` to transform the input before passing it to `eval_expr`. We should focus on making `stringify_expr` safer instead.
        
        3. The `sympify` function is a good place to add additional checks, but we need to be careful not to break existing functionality.
        
        4. The `__eq__` method in `Expr` class doesn't directly call `eval` or `sympify`, so it's not the root cause of the issue. However, it does use `sympify`, so it will benefit from the changes we make to `sympify`.
        
        5. We should add a new step to modify the `stringify_expr` function to make it safer.
        
        Let's revise the plan with these considerations in mind.
        </thinking>
        
        <symbol_list>
        <symbol>
        <name>eval_expr</name>
        <file_path>sympy/parsing/sympy_parser.py</file_path>
        <plan>
        Modify eval_expr to use a safer evaluation method:
        1. Instead of using Python's built-in eval, use ast.literal_eval which only evaluates literals.
        2. If ast.literal_eval fails, fall back to a custom safe_eval function that only allows specific SymPy operations.
        3. Wrap the evaluation in a try-except block to catch any potential security-related exceptions.
        </plan>
        </symbol>
        
        <symbol>
        <name>stringify_expr</name>
        <file_path>sympy/parsing/sympy_parser.py</file_path>
        <plan>
        Modify stringify_expr to add additional safety checks:
        1. Add a whitelist of allowed SymPy functions and symbols.
        2. Check each token against the whitelist before including it in the output.
        3. If an unknown token is encountered, raise a SecurityError instead of including it in the output.
        </plan>
        </symbol>
        
        <symbol>
        <name>sympify</name>
        <file_path>sympy/core/sympify.py</file_path>
        <plan>
        Update sympify to add more stringent checks:
        1. If the input is already a SymPy type, return it immediately.
        2. If the input is a string, check if it's a valid SymPy expression using a new is_valid_sympy_expr function before passing it to parse_expr.
        3. If the input is neither a SymPy type nor a valid SymPy expression string, raise a SympifyError.
        </plan>
        </symbol>
        
        <symbol>
        <name>__eq__</name>
        <file_path>sympy/core/expr.py</file_path>
        <plan>
        The __eq__ method doesn't need to be modified directly, but we should add a test to ensure it behaves correctly with the new safety measures:
        1. Add a test case that compares a SymPy expression with an object that has a potentially harmful __repr__ method.
        2. Verify that the comparison raises a SympifyError instead of evaluating the repr.
        </plan>
        </symbol>
        </symbol_list>
        </reply>
        "#;

        let parsed_response = PlanningBeforeCodeEditResponse::parse_response(&response);
        assert!(parsed_response.is_ok());
    }

    #[test]
    fn test_with_extra_data() {
        let output = r#"
        <reply>
        <thinking>
        After analyzing the code and the original plan, I believe some modifications and additions are necessary to properly address the issue:
        
        1. The `eval_expr` function is not the root cause of the problem. The issue stems from `sympify` attempting to parse unknown objects.
        
        2. We need to focus on `sympify` in sympy/core/sympify.py, as it's the function that handles conversion of objects to SymPy types.
        
        3. The `__eq__` method in sympy/core/expr.py calls `sympify`, so we don't need to modify it directly. The changes in `sympify` will propagate to `__eq__`.
        
        4. We should add a new function to safely convert objects to SymPy types without using `eval`.
        
        5. The `parse_expr` function should be modified to use this new safe conversion method.
        
        Here's the revised plan:
        </thinking>
        
        <symbol_list>
        <symbol>
        <name>sympify</name>
        <file_path>sympy/core/sympify.py</file_path>
        <plan>
        1. Modify `sympify` to check if the argument is already a SymPy type before attempting to convert it.
        2. If the argument is not a SymPy type, use a new safe conversion method instead of `parse_expr`.
        3. Raise a `SympifyError` if the safe conversion fails.
        </plan>
        </symbol>
        
        <symbol>
        <name>safe_sympify</name>
        <file_path>sympy/core/sympify.py</file_path>
        <plan>
        1. Implement a new function `safe_sympify` that safely converts objects to SymPy types.
        2. This function should handle basic Python types (int, float, complex, str) and SymPy types.
        3. For other types, it should return the object unchanged or raise a `SympifyError`.
        </plan>
        </symbol>
        
        <symbol>
        <name>parse_expr</name>
        <file_path>sympy/parsing/sympy_parser.py</file_path>
        <plan>
        1. Modify `parse_expr` to use the new `safe_sympify` function for initial conversion.
        2. Only apply transformations and parsing if the input is a string.
        3. Raise an error if an unknown object type is passed.
        </plan>
        </symbol>
        
        <symbol>
        <name>__eq__</name>
        <file_path>sympy/core/expr.py</file_path>
        <plan>
        No direct changes needed. The modifications to `sympify` will ensure that `__eq__` behaves safely when comparing with unknown objects.
        </plan>
        </symbol>
        </symbol_list>
        
        <extra_data>
        We should also consider adding or updating test cases in the appropriate test files to ensure the new behavior is correct and the security issue is resolved.
        </extra_data>
        </reply>
        "#;
        let parsed_response = PlanningBeforeCodeEditResponse::parse_response(&output);
        println!("{:?}", &parsed_response);
        assert!(parsed_response.is_ok());
    }
}
