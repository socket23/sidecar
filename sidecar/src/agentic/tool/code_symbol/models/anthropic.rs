use async_trait::async_trait;
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer, Serializer,
};
use serde_xml_rs::from_str;
use std::{borrow::Cow, fmt, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::agentic::tool::code_symbol::{
    important::{
        CodeSymbolImportant, CodeSymbolImportantRequest, CodeSymbolImportantResponse,
        CodeSymbolImportantWideSearch, CodeSymbolWithSteps, CodeSymbolWithThinking,
    },
    types::CodeSymbolError,
};

pub struct AnthropicCodeSymbolImportant {
    llm_client: Arc<LLMBroker>,
}

impl AnthropicCodeSymbolImportant {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename = "name")]
pub struct SymbolName {
    #[serde(rename = "$value")]
    name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename = "thinking")]
pub struct SymbolThinking {
    #[serde(rename = "$value")]
    thinking: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename = "symbol")]
pub struct Symbol {
    name: String,
    thinking: String,
    file_path: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename = "step_list")]
pub struct StepListItem {
    name: String,
    step: Vec<String>,
    #[serde(default)]
    new: bool,
    file_path: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename = "symbol_list")]
pub struct SymbolList {
    #[serde(rename = "$value")]
    symbol_list: Vec<Symbol>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename = "step_by_step")]
pub struct StepList {
    #[serde(rename = "$value")]
    steps: Vec<StepListItem>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename = "reply")]
pub struct Reply {
    symbol_list: SymbolList,
    // #[serde(rename = "step_by_step")]
    step_by_step: StepList,
}

impl Reply {
    pub fn fix_escaped_string(self) -> Self {
        let step_by_step = self
            .step_by_step
            .steps
            .into_iter()
            .map(|step| {
                let steps = step
                    .step
                    .into_iter()
                    .map(|step| AnthropicCodeSymbolImportant::escape_xml(step))
                    .collect();
                StepListItem {
                    name: step.name,
                    step: steps,
                    new: step.new,
                    file_path: step.file_path,
                }
            })
            .collect::<Vec<_>>();
        Self {
            symbol_list: self.symbol_list,
            step_by_step: StepList {
                steps: step_by_step,
            },
        }
    }
}

impl Reply {
    fn to_code_symbol_important_response(self) -> CodeSymbolImportantResponse {
        let code_symbols_with_thinking = self
            .symbol_list
            .symbol_list
            .into_iter()
            .map(|symbol_list| {
                CodeSymbolWithThinking::new(
                    symbol_list.name,
                    symbol_list.thinking,
                    symbol_list.file_path,
                )
            })
            .collect();
        let code_symbols_with_steps = self
            .step_by_step
            .steps
            .into_iter()
            .map(|step| CodeSymbolWithSteps::new(step.name, step.step, step.new, step.file_path))
            .collect();
        CodeSymbolImportantResponse::new(code_symbols_with_thinking, code_symbols_with_steps)
    }
}

impl AnthropicCodeSymbolImportant {
    pub fn system_message_context_wide(
        &self,
        code_symbol_search_context_wide: &CodeSymbolImportantWideSearch,
    ) -> String {
        format!(
            r#"You are a search engine which makes no mistakes while retriving important context for a user-query.
You will be given context which the user has selected in <user_context> and you have to retrive the "code symbols" which are important for answering to the user query.
- The user might have selected some context manually in the form of <selection> these might be more important
- You will be given files which contains a lot of code, you have to select the "code symbols" which are important
- "code symbols" here referes to the different classes, functions, or constants which might be necessary to answer the user query.
- Now you will write a step by step process for making the code edit, this ensures that you lay down the plan before making the change, put this in an xml section called <step_by_step> where each step is in <step_item> section where each section has the name of the symbol on which the operation will happen, if no such symbol exists and you need to create a new one put a <new>true</new> inside the step section and after the symbols
- In your step by step list make sure that the symbols are listed in the order in which we have to go about making the changes
- Strictly followt he reply format which is mentioned to you below, your reply should always start with <reply> tag and end with </reply> tag

Let's focus on getting the "code symbols" which are necessary to satisfy the user query.

As an example, given the following code selection:
<code_selection>
<file_path>
sidecar/broker/fill_in_middle.rs
</file_path>
```rust
pub struct FillInMiddleBroker {{
    providers: HashMap<LLMType, Box<dyn FillInMiddleFormatter + Send + Sync>>,
}}

impl FillInMiddleBroker {{
    pub fn new() -> Self {{
        let broker = Self {{
            providers: HashMap::new(),
        }};
        broker
            .add_llm(
                LLMType::CodeLlama13BInstruct,
                Box::new(CodeLlamaFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::CodeLlama7BInstruct,
                Box::new(CodeLlamaFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder1_3BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder6BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder33BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeHaiku,
                Box::new(ClaudeFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeOpus,
                Box::new(ClaudeFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeSonnet,
                Box::new(ClaudeFillInMiddleFormatter::new()),
            )
    }}
```
</code_selection>

and the user query is:
<user_query>
I want to add support for the grok llm
</user_query>

Your reply should be, you should strictly follow this format:
<reply>
<symbol_list>
<symbol>
<name>
LLMType
</name>
<file_path>
sidecar/broker/fill_in_middle.rs
</file_path>
<thinking>
We need to first check if grok is part of the LLMType enum, this will make sure that the code we produce is never wrong
</thinking>
</symbol>
<symbol>
<name>
FillInMiddleFormatter
</name>
<file_path>
sidecar/broker/fill_in_middle.rs
</file_path>
<thinking>
Other LLM's are implementing FillInMiddleFormatter trait, grok will also require support for this, so we need to check how to implement FillInMiddleFormatter trait
</thinking>
</symbol>
<symbol>
<name>
new
</name>
<file_path>
sidecar/broker/fill_in_middle.rs
</file_path>
<thinking>
We have to change the new function and add the grok llm after implementing the formatter for grok llm.
</thinking>
</symbol>
</symbol_list>
<step_by_step>
<step_list>
<name>
LLMType
</name>
<file_path>
sidecar/broker/fill_in_middle.rs
</file_path>
<step>
We will need to first check the LLMType if it has support for grok or we need to edit it first
</step>
</step_list>
<step_list>
<name>
FillInMiddleFormatter
</name>
<file_path>
sidecar/broker/fill_in_middle.rs
</file_path>
<step>
Check the definition of `FillInMiddleFormatter` to see how to implement it
</step>
</step_list>
<step_list
<name>
CodeLlamaFillInMiddleFormatter
</name>
<file_path>
sidecar/broker/fill_in_middle.rs
</file_path>
<step>
We can follow the implementation of CodeLlamaFillInMiddleFormatter since we will also have to follow a similar pattern of making changes and adding it to the right places if there are more.
</step>
</step_list>
<step_list>
<name>
GrokFillInMiddleFormatter
</name>
<file_path>
sidecar/broker/fill_in_middle.rs
</file_path>
<new>
true
</new>
<step>
Implement the GrokFillInMiddleFormatter following the similar pattern in `CodeLlamaFillInMiddleFormatter`
</step>
</step_list>
</step_by_step>
</reply>

Another example:
<code_selection>
```rust
fn tree_sitter_router() -> Router {{
    use axum::routing::*;
    Router::new()
        .route(
            "/documentation_parsing",
            post(sidecar::webserver::tree_sitter::extract_documentation_strings),
        )
        .route(
            "/diagnostic_parsing",
            post(sidecar::webserver::tree_sitter::extract_diagnostics_range),
        )
        .route(
            "/tree_sitter_valid",
            post(sidecar::webserver::tree_sitter::tree_sitter_node_check),
        )
}}

fn file_operations_router() -> Router {{
    use axum::routing::*;
    Router::new().route("/edit_file", post(sidecar::webserver::file_edit::file_edit))
}}

fn inline_completion() -> Router {{
    use axum::routing::*;
    Router::new()
        .route(
            "/inline_completion",
            post(sidecar::webserver::inline_completion::inline_completion),
        )
        .route(
            "/cancel_inline_completion",
            post(sidecar::webserver::inline_completion::cancel_inline_completion),
        )
        .route(
            "/document_open",
            post(sidecar::webserver::inline_completion::inline_document_open),
        )
        .route(
            "/document_content_changed",
            post(sidecar::webserver::inline_completion::inline_completion_file_content_change),
        )
        .route(
            "/get_document_content",
            post(sidecar::webserver::inline_completion::inline_completion_file_content),
        )
        .route(
            "/get_identifier_nodes",
            post(sidecar::webserver::inline_completion::get_identifier_nodes),
        )
        .route(
            "/get_symbol_history",
            post(sidecar::webserver::inline_completion::symbol_history),
        )
}}

// TODO(skcd): Figure out why we are passing the context in the suffix and not the prefix

```
</code_selection>

and the user query is:
<user_query>
I want to get the list of most important symbols in inline completions
</user_query>

Your reply should be:
<reply>
<symbol_list>
<symbol>
<name>
inline_completion
</name>
<thinking>
inline_completion holds all the endpoints for symbols because it also has the `get_symbol_history` endpoint. We have to start adding the endpoint there
</thinking>
</symbol>
<symbol>
<name>
symbol_history
</name>
<thinking>
I can find more information on how to write the code for the endpoint by following the symbol `symbol_history` in the line: `             post(sidecar::webserver::inline_completion::symbol_history),`
<thinking>
</symbol>
</symbol_list>
<step_by_step>
<step_list>
<name>
symbol_history
</name>
<thinking>
We need to follow the symbol_history to check the pattern on how we are going to implement the very similar functionality
</thinking>
</step_list>
<step_list>
<name>
inline_completion
</name>
<thinking>
We have to add the newly created endpoint in inline_completion to add support for the new endpoint which we want to create
</thinking>
</step_list>
</step_by_step>
</reply>"#
        )
    }
    pub fn system_message(
        &self,
        code_symbol_important_request: &CodeSymbolImportantRequest,
    ) -> String {
        if code_symbol_important_request.symbol_identifier().is_some() {
            todo!("we need to figure it out")
        } else {
            format!(
                r#"You are an expert software engineer who makes no mistakes while writing code
- The user has selected some code, before you start making changes you select the most important symbols which you need to either change or follow along for the context.
- Get more context about the different symbols such as classes, functions, enums, types (and more), this ensures that you are able to gather everything necessary before making the code edit and the code you write will not use any wrong code out of this selection.
- Now you will write a step by step process for making the code edit, this ensures that you lay down the plan before making the change, put this in an xml section called <step_by_step> where each step is in <step_item> section where each section has the name of the symbol on which the operation will happen, if no such symbol exists and you need to create a new one put a <new>true</new> inside the step section and after the symbols
- In your step by step list make sure that the symbols are listed in the order in which we have to go about making the changes
- Strictly follow the reply format which is mentioned to you below, your reply should always start with <reply> tag and end with </reply> tag

For now let's focus on the first step, gathering all the required symbol definitions and types.

As an example, given the following code selection:
<code_selection>
```rust
pub struct FillInMiddleBroker {{
    providers: HashMap<LLMType, Box<dyn FillInMiddleFormatter + Send + Sync>>,
}}

impl FillInMiddleBroker {{
    pub fn new() -> Self {{
        let broker = Self {{
            providers: HashMap::new(),
        }};
        broker
            .add_llm(
                LLMType::CodeLlama13BInstruct,
                Box::new(CodeLlamaFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::CodeLlama7BInstruct,
                Box::new(CodeLlamaFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder1_3BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder6BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::DeepSeekCoder33BInstruct,
                Box::new(DeepSeekFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeHaiku,
                Box::new(ClaudeFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeOpus,
                Box::new(ClaudeFillInMiddleFormatter::new()),
            )
            .add_llm(
                LLMType::ClaudeSonnet,
                Box::new(ClaudeFillInMiddleFormatter::new()),
            )
    }}
```
</code_selection>

and the user query is:
<user_query>
I want to add support for the grok llm
</user_query>

Your reply should be, you should strictly follow this format:
<reply>
<symbol_list>
<symbol>
<name>
LLMType
</name>
<thinking>
We need to first check if grok is part of the LLMType enum, this will make sure that the code we produce is never wrong
</thinking>
</symbol>
<symbol>
<name>
FillInMiddleFormatter
</name>
<thinking>
Other LLM's are implementing FillInMiddleFormatter trait, grok will also require support for this, so we need to check how to implement FillInMiddleFormatter trait
</thinking>
</symbol>
<symbol>
<name>
new
</name>
<thinking>
We have to change the new function and add the grok llm after implementing the formatter for grok llm.
</thinking>
</symbol>
</symbol_list>
<step_by_step>
<step_list>
<name>
LLMType
</name>
<step>
We will need to first check the LLMType if it has support for grok or we need to edit it first
</step>
</step_list>
<step_list>
<name>
FillInMiddleFormatter
</name>
<step>
Check the definition of `FillInMiddleFormatter` to see how to implement it
</step>
</step_list>
<step_list
<name>
CodeLlamaFillInMiddleFormatter
</name>
<step>
We can follow the implementation of CodeLlamaFillInMiddleFormatter since we will also have to follow a similar pattern of making changes and adding it to the right places if there are more.
</step>
</step_list>
<step_list>
<name>
GrokFillInMiddleFormatter
</name>
<new>
true
</new>
<step>
Implement the GrokFillInMiddleFormatter following the similar pattern in `CodeLlamaFillInMiddleFormatter`
</step>
</step_list>
</step_by_step>
</reply>

Another example:
<code_selection>
```rust
fn tree_sitter_router() -> Router {{
    use axum::routing::*;
    Router::new()
        .route(
            "/documentation_parsing",
            post(sidecar::webserver::tree_sitter::extract_documentation_strings),
        )
        .route(
            "/diagnostic_parsing",
            post(sidecar::webserver::tree_sitter::extract_diagnostics_range),
        )
        .route(
            "/tree_sitter_valid",
            post(sidecar::webserver::tree_sitter::tree_sitter_node_check),
        )
}}

fn file_operations_router() -> Router {{
    use axum::routing::*;
    Router::new().route("/edit_file", post(sidecar::webserver::file_edit::file_edit))
}}

fn inline_completion() -> Router {{
    use axum::routing::*;
    Router::new()
        .route(
            "/inline_completion",
            post(sidecar::webserver::inline_completion::inline_completion),
        )
        .route(
            "/cancel_inline_completion",
            post(sidecar::webserver::inline_completion::cancel_inline_completion),
        )
        .route(
            "/document_open",
            post(sidecar::webserver::inline_completion::inline_document_open),
        )
        .route(
            "/document_content_changed",
            post(sidecar::webserver::inline_completion::inline_completion_file_content_change),
        )
        .route(
            "/get_document_content",
            post(sidecar::webserver::inline_completion::inline_completion_file_content),
        )
        .route(
            "/get_identifier_nodes",
            post(sidecar::webserver::inline_completion::get_identifier_nodes),
        )
        .route(
            "/get_symbol_history",
            post(sidecar::webserver::inline_completion::symbol_history),
        )
}}

// TODO(skcd): Figure out why we are passing the context in the suffix and not the prefix

```
</code_selection>

and the user query is:
<user_query>
I want to get the list of most important symbols in inline completions
</user_query>

Your reply should be:
<reply>
<symbol_list>
<symbol>
<name>
inline_completion
</name>
<thinking>
inline_completion holds all the endpoints for symbols because it also has the `get_symbol_history` endpoint. We have to start adding the endpoint there
</thinking>
</symbol>
<symbol>
<name>
symbol_history
</name>
<thinking>
I can find more information on how to write the code for the endpoint by following the symbol `symbol_history` in the line: `             post(sidecar::webserver::inline_completion::symbol_history),`
<thinking>
</symbol>
</symbol_list>
<step_by_step>
<step_list>
<name>
symbol_history
</name>
<thinking>
We need to follow the symbol_history to check the pattern on how we are going to implement the very similar functionality
</thinking>
</step_list>
<step_list>
<name>
inline_completion
</name>
<thinking>
We have to add the newly created endpoint in inline_completion to add support for the new endpoint which we want to create
</thinking>
</step_list>
</step_by_step>
</reply>"#
            )
        }
    }

    fn user_message(&self, code_symbols: &CodeSymbolImportantRequest) -> String {
        let query = code_symbols.query();
        let file_path = code_symbols.file_path();
        let language = code_symbols.language();
        let lines = code_symbols
            .content()
            .lines()
            .enumerate()
            .collect::<Vec<(usize, _)>>();
        let selection_range = code_symbols.range();
        let line_above = (selection_range.start_line() as i64) - 1;
        let line_below = (selection_range.end_line() as i64) + 1;
        let code_above = lines
            .iter()
            .filter(|(line_number, _)| *line_number as i64 <= line_above)
            .map(|(_, line)| *line)
            .collect::<Vec<&str>>()
            .join("\n");
        let code_below = lines
            .iter()
            .filter(|(line_number, _)| *line_number as i64 >= line_below)
            .map(|(_, line)| *line)
            .collect::<Vec<&str>>()
            .join("\n");
        let code_selection = lines
            .iter()
            .filter(|(line_number, _)| {
                *line_number as i64 >= selection_range.start_line() as i64
                    && *line_number as i64 <= selection_range.end_line() as i64
            })
            .map(|(_, line)| *line)
            .collect::<Vec<&str>>()
            .join("\n");
        if code_symbols.symbol_identifier().is_none() {
            format!(
                r#"<file_path>
{file_path}
</file_path>
<code_above>
```{language}
{code_above}
```
</code_above>
<code_below>
```{language}
{code_below}
```
</code_below>
<code_selection>
```{language}
{code_selection}
```
</code_selection>
<user_query>
{query}
</user_query>"#
            )
        } else {
            format!("")
        }
    }

    fn unescape_xml(s: String) -> String {
        s.replace("\"", "&quot;")
            .replace("'", "&apos;")
            .replace(">", "&gt;")
            .replace("<", "&lt;")
            .replace("&", "&amp;")
    }

    fn escape_xml(s: String) -> String {
        s.replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&gt;", ">")
            .replace("&lt;", "<")
            .replace("&amp;", "&")
    }

    // Welcome to the jungle and an important lesson on why xml sucks sometimes
    // &, and < are invalid characters in xml, so we can't simply parse it using
    // serde cause the xml parser will legit look at these symbols and fail
    // hard, instead we have to escape these strings properly
    // and not at everypace (see it gets weird)
    // we only have to do this inside the <step>{content}</step> tags
    // so lets get to it
    // one good thing we know is that because we ask claude to follow this format
    // it will always give a new line so we can just split into lines and then
    // do the replacement
    fn cleanup_string(response: &str) -> String {
        let mut is_inside_step = false;
        let mut new_lines = vec![];
        for line in response.lines() {
            if line == "<step>" {
                is_inside_step = true;
                new_lines.push("<step>".to_owned());
                continue;
            } else if line == "</step>" {
                is_inside_step = false;
                new_lines.push("</step>".to_owned());
                continue;
            }
            if is_inside_step {
                new_lines.push(Self::unescape_xml(line.to_owned()))
            } else {
                new_lines.push(line.to_owned());
            }
        }
        new_lines.join("\n")
    }

    fn parse_response(response: &str) -> Result<Reply, CodeSymbolError> {
        let parsed_response = Self::cleanup_string(response);
        // we want to grab the section between <reply> and </reply> tags
        // and then we want to parse the response which is in the following format
        let lines = parsed_response
            .lines()
            .skip_while(|line| !line.contains("<reply>"))
            .skip(1)
            .take_while(|line| !line.contains("</reply>"))
            .collect::<Vec<&str>>()
            .join("\n");
        let reply = format!(
            r#"<reply>
{lines}
</reply>"#
        );
        Ok(from_str::<Reply>(&reply)
            .map(|reply| reply.fix_escaped_string())
            .map_err(|e| CodeSymbolError::SerdeError(e))?)
    }

    async fn user_message_for_codebase_wide_search(
        &self,
        code_symbol_search_context_wide: CodeSymbolImportantWideSearch,
    ) -> Result<String, CodeSymbolError> {
        let user_query = code_symbol_search_context_wide.user_query().to_owned();
        let user_context = code_symbol_search_context_wide.remove_user_context();
        let context_string = user_context
            .to_xml()
            .await
            .map_err(|e| CodeSymbolError::UserContextError(e))?;
        // also send the user query here
        Ok(context_string + "\n" + "<user_query>\n" + &user_query + "\n</user_query>")
    }
}

#[async_trait]
impl CodeSymbolImportant for AnthropicCodeSymbolImportant {
    async fn get_important_symbols(
        &self,
        code_symbols: CodeSymbolImportantRequest,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError> {
        if !code_symbols.model().is_anthropic() {
            return Err(CodeSymbolError::WrongLLM(code_symbols.model().clone()));
        }
        let system_message = LLMClientMessage::system(self.system_message(&code_symbols));
        let user_message = LLMClientMessage::user(self.user_message(&code_symbols));
        let messages = LLMClientCompletionRequest::new(
            code_symbols.model().clone(),
            vec![system_message, user_message],
            0.0,
            None,
        );
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let response = self
            .llm_client
            .stream_completion(
                code_symbols.api_key().clone(),
                messages,
                code_symbols.provider().clone(),
                vec![("request_type".to_owned(), "important_symbols".to_owned())]
                    .into_iter()
                    .collect(),
                sender,
            )
            .await
            .map_err(|e| CodeSymbolError::LLMClientError(e))?;

        Self::parse_response(&response).map(|reply| reply.to_code_symbol_important_response())
    }

    async fn context_wide_search(
        &self,
        code_symbols: CodeSymbolImportantWideSearch,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError> {
        if !code_symbols.model().is_anthropic() {
            return Err(CodeSymbolError::WrongLLM(code_symbols.model().clone()));
        }
        let api_key = code_symbols.api_key();
        let provider = code_symbols.llm_provider();
        let model = code_symbols.model().clone();
        let system_message =
            LLMClientMessage::system(self.system_message_context_wide(&code_symbols));
        let user_message = LLMClientMessage::user(
            self.user_message_for_codebase_wide_search(code_symbols)
                .await?,
        );
        let messages =
            LLMClientCompletionRequest::new(model, vec![system_message, user_message], 0.0, None);
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let response = self
            .llm_client
            .stream_completion(
                api_key,
                messages,
                provider,
                vec![("request_type".to_owned(), "context_wide_search".to_owned())]
                    .into_iter()
                    .collect(),
                sender,
            )
            .await?;
        Self::parse_response(&response).map(|reply| reply.to_code_symbol_important_response())
    }
}

#[cfg(test)]
mod tests {

    use serde_xml_rs::from_str;

    use super::{AnthropicCodeSymbolImportant, StepListItem};

    #[test]
    fn test_parsing_works_for_important_symbol() {
        let reply = r#"<reply>
<symbol_list>
<symbol>
<name>
LLMProvider
</name>
<file_path>
/Users/skcd/scratch/sidecar/llm_client/src/provider.rs
</file_path>
<thinking>
We need to first add a new variant to the LLMProvider enum to represent the GROQ provider.
</thinking>
</symbol>
<symbol>
<name>
LLMProviderAPIKeys
</name>
<file_path>
/Users/skcd/scratch/sidecar/llm_client/src/provider.rs
</file_path>
<thinking>
We also need to add a new variant to the LLMProviderAPIKeys enum to hold the API key for the GROQ provider.
</thinking>
</symbol>
<symbol>
<name>
LLMBroker
</name>
<file_path>
/Users/skcd/scratch/sidecar/llm_client/src/broker.rs
</file_path>
<thinking>
We need to update the LLMBroker to add support for the new GROQ provider. This includes adding a new case in the get_provider function and adding a new provider to the providers HashMap.
</thinking>
</symbol>
<symbol>
<new>
true
</new>
<name>
GroqClient
</name>
<file_path>
/Users/skcd/scratch/sidecar/llm_client/src/clients/groq.rs
</file_path>
<thinking>
We need to create a new GroqClient struct that implements the LLMClient trait. This client will handle communication with the GROQ provider.
</thinking>
</symbol>
</symbol_list>
<step_by_step>
<step_list>
<name>
LLMProvider
</name>
<file_path>
/Users/skcd/scratch/sidecar/llm_client/src/provider.rs
</file_path>
<step>
Add a new variant to the LLMProvider enum to represent the GROQ provider:

```rust
pub enum LLMProvider {
    // ...
    Groq,
    // ...
}
```
</step>
</step_list>
<step_list>
<name>
LLMProviderAPIKeys
</name>
<file_path>
/Users/skcd/scratch/sidecar/llm_client/src/provider.rs
</file_path>
<step>
Add a new variant to the LLMProviderAPIKeys enum to hold the API key for the GROQ provider:

```rust
pub enum LLMProviderAPIKeys {
    // ...
    Groq(GroqAPIKey),
    // ...
}
```

Create a new struct to hold the API key for the GROQ provider:

```rust
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GroqAPIKey {
    pub api_key: String,
    // Add any other necessary fields
}
```
</step>
</step_list>
<step_list>
<name>
LLMBroker
</name>
<file_path>
/Users/skcd/scratch/sidecar/llm_client/src/broker.rs
</file_path>
<step>
Update the get_provider function in the LLMBroker to handle the new GROQ provider:

```rust
fn get_provider(&self, api_key: &LLMProviderAPIKeys) -> LLMProvider {
    match api_key {
        // ...
        LLMProviderAPIKeys::Groq(_) => LLMProvider::Groq,
        // ...
    }
}
```

Add a new case in the stream_completion and stream_string_completion functions to handle the GROQ provider:

```rust
pub async fn stream_completion(
    &self,
    api_key: LLMProviderAPIKeys,
    request: LLMClientCompletionRequest,
    provider: LLMProvider,
    metadata: HashMap<String, String>,
    sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
) -> LLMBrokerResponse {
    // ...
    let provider_type = match &api_key {
        // ...
        LLMProviderAPIKeys::Groq(_) => LLMProvider::Groq,
        // ...
    };
    // ...
}

pub async fn stream_string_completion(
    &self,
    api_key: LLMProviderAPIKeys,
    request: LLMClientCompletionStringRequest,
    metadata: HashMap<String, String>,
    sender: tokio::sync::mpsc::UnboundedSender<LLMClientCompletionResponse>,
) -> LLMBrokerResponse {
    // ...
    let provider_type = match &api_key {
        // ...
        LLMProviderAPIKeys::Groq(_) => LLMProvider::Groq,
        // ...
    };
    // ...
}
```

In the LLMBroker::new function, add the new GROQ provider to the providers HashMap:

```rust
pub async fn new(config: LLMBrokerConfiguration) -> Result<Self, LLMClientError> {
    // ...
    Ok(broker
        // ...
        .add_provider(LLMProvider::Groq, Box::new(GroqClient::new())))
}
```
</step>
</step_list>
</step_by_step>
</reply>"#;

        let parsed_response = AnthropicCodeSymbolImportant::parse_response(reply);
        assert!(parsed_response.is_ok());
    }
}
