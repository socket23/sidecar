use async_trait::async_trait;
use quick_xml::de::from_str;
use std::sync::Arc;

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
pub struct NewSubSymbolRequiredRequest {
    user_query: String,
    plan: String,
    symbol_content: String,
    llm_properties: LLMProperties,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "symbol")]
pub struct NewSymbol {
    symbol_name: String,
    reason_to_create: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "new_symbols")]
pub struct NewSubSymbolRequiredResponse {
    #[serde(rename = "$value")]
    symbols: Vec<NewSymbol>,
}

impl NewSubSymbolRequiredResponse {
    fn unescape_thinking_string(self) -> Self {
        let fixed_symbols = self
            .symbols
            .into_iter()
            .map(|symbol| {
                let symbol_name = symbol.symbol_name;
                let reason_to_create = symbol
                    .reason_to_create
                    .lines()
                    .map(|line| unescape_xml(line.to_owned()))
                    .collect::<Vec<_>>()
                    .join("\n");
                NewSymbol {
                    symbol_name,
                    reason_to_create,
                }
            })
            .collect();
        Self {
            symbols: fixed_symbols,
        }
    }
    fn parse_response(response: &str) -> Result<Self, ToolError> {
        let tags_to_exist = vec!["<reply>", "</reply>", "<new_symbols>", "</new_symbols>"];
        if tags_to_exist.into_iter().any(|tag| !response.contains(tag)) {
            return Err(ToolError::MissingXMLTags);
        }
        let lines = response
            .lines()
            .skip_while(|line| !line.contains("<new_symbols>"))
            .skip(1)
            .take_while(|line| !line.contains("</new_symbols>"))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = format!(
            r#"<new_symbols>
{lines}
</new_symbols>"#
        );

        let mut final_lines = vec![];
        let mut is_inside = false;
        for line in lines.lines() {
            if line == "<thinking>" {
                is_inside = true;
                final_lines.push(line.to_owned());
                continue;
            } else if line == "</thinking>" {
                is_inside = false;
                final_lines.push(line.to_owned());
                continue;
            }
            if is_inside {
                final_lines.push(escape_xml(line.to_owned()));
            } else {
                final_lines.push(line.to_owned());
            }
        }

        let parsed_response = from_str::<NewSubSymbolRequiredResponse>(&final_lines.join("\n"));
        match parsed_response {
            Err(_e) => Err(ToolError::SerdeConversionFailed),
            Ok(parsed_list) => Ok(parsed_list.unescape_thinking_string()),
        }
    }
}

pub struct NewSubSymbolRequired {
    llm_client: Arc<LLMBroker>,
}

impl NewSubSymbolRequired {
    pub fn new(llm_client: Arc<LLMBroker>) -> Self {
        Self { llm_client }
    }

    pub fn system_message(&self) -> String {
        r#"You are an expert software engineer who is an expert at figuring out if we need to create new functions inside the code symbol or if existing functions can be edited to satify the user query.
- You will be given the original user query in <user_query>
- You will be provided the code symbol in <code_symbol> section.
- The plan of edits which we want to do on this code symbol is also given in <plan> section.
- You have to decide if we can make changes to the existing functions inside this code symbol or if we need to create new functions which will belong to this code symbol.
- Before replying, think step-by-step on what approach we want to take and put your thinking in <thinking> section.
Your reply should be in the following format:
<reply>
<thinking>
{{your thinking process before replying}}
</thinking>
<new_symbols>
<symbol>
<symbol_name>
{{name of the symbol}}
</symbol_name>
<reason_to_create>
{{your reason for creating this new symbol inside the main symbol}}
</reason_to_create>
</symbol>
{{... more symbols which should belong in the list}}
</new_symbols>
</reply>

Please make sure to keep your reply in the <reply> tag and the new symbols which you need to generate properly in the format under <new_symbols> section."#.to_owned()
    }

    pub fn user_message(&self, request: NewSubSymbolRequiredRequest) -> String {
        let user_query = request.user_query;
        let plan = request.plan;
        let symbol_content = request.symbol_content;
        format!(
            r#"<user_query>
{user_query}
</user_query>

<plan>
{plan}
</plan>

<symbol_content>
{symbol_content}
</symbol_content>"#
        )
    }
}

#[async_trait]
impl Tool for NewSubSymbolRequired {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.get_new_sub_symbol_for_code_editing()?;
        let llm_properties = context.llm_properties.clone();
        let system_message = LLMClientMessage::system(self.system_message());
        let user_message = LLMClientMessage::user(self.user_message(context));
        let llm_request = LLMClientCompletionRequest::new(
            llm_properties.llm().clone(),
            vec![system_message, user_message],
            0.2,
            None,
        );
        let mut retries = 0;
        loop {
            if retries >= 4 {
                return Err(ToolError::MissingXMLTags);
            }
            let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
            let response = self
                .llm_client
                .stream_completion(
                    llm_properties.api_key().clone(),
                    llm_request.clone(),
                    llm_properties.provider().clone(),
                    vec![(
                        "event_type".to_owned(),
                        "new_sub_sybmol_required".to_owned(),
                    )]
                    .into_iter()
                    .collect(),
                    sender,
                )
                .await;
            match response {
                Ok(response) => {
                    if let Ok(parsed_response) =
                        NewSubSymbolRequiredResponse::parse_response(&response)
                    {
                        return Ok(ToolOutput::new_sub_symbol_creation(parsed_response));
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
