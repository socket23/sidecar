use async_trait::async_trait;
use serde_xml_rs::from_str;
use std::{collections::HashMap, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::agentic::{
    symbol::identifier::Snippet,
    tool::{
        base::{Tool, ToolType},
        errors::ToolError,
        filtering::errors::CodeToEditFilteringError,
        input::ToolInput,
        output::ToolOutput,
    },
};

use super::models::anthropic::AnthropicCodeToEditFormatter;

#[derive(Debug, Clone)]
pub struct SnippetWithReason {
    snippet: Snippet,
    reason: String,
}

impl SnippetWithReason {
    pub fn new(snippet: Snippet, reason: String) -> Self {
        Self { snippet, reason }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn remove_snippet(self) -> Snippet {
        self.snippet
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_probe")]
pub struct CodeToProbeSnippet {
    id: usize,
    reason_to_probe: String,
}

impl CodeToProbeSnippet {
    pub fn id(&self) -> usize {
        self.id.clone()
    }

    pub fn reason_to_probe(&self) -> &str {
        &self.reason_to_probe
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_edit")]
pub struct CodeToEditSnippet {
    id: usize,
    reason_to_edit: String,
}

impl CodeToEditSnippet {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn reason_to_edit(&self) -> &str {
        &self.reason_to_edit
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_not_probe")]
pub struct CodeToNotProbeSnippet {
    id: usize,
    reason_to_not_probe: String,
}

impl CodeToNotProbeSnippet {
    pub fn id(&self) -> usize {
        self.id.clone()
    }

    pub fn reason_to_no_probe(&self) -> &str {
        &self.reason_to_not_probe
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_not_edit")]
pub struct CodeToNotEditSnippet {
    id: usize,
    reason_to_not_edit: String,
}

impl CodeToNotEditSnippet {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn reason_to_not_edit(&self) -> &str {
        &self.reason_to_not_edit
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_probe_list")]
pub struct CodeToProbeList {
    #[serde(default, rename = "$value")]
    snippets: Vec<CodeToProbeSnippet>,
}

impl CodeToProbeList {
    pub fn snippets(&self) -> &[CodeToProbeSnippet] {
        self.snippets.as_slice()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_edit_list")]
pub struct CodeToEditList {
    #[serde(rename = "$value")]
    snippets: Vec<CodeToEditSnippet>,
}

impl CodeToEditList {
    pub fn snippets(&self) -> &[CodeToEditSnippet] {
        self.snippets.as_slice()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename = "code_to_not_edit_list")]
pub struct CodeToNotEditList {
    #[serde(default, rename = "$value")]
    snippets: Vec<CodeToNotEditSnippet>,
}

impl CodeToNotEditList {
    pub fn snippets(&self) -> &[CodeToNotEditSnippet] {
        self.snippets.as_slice()
    }
}

#[derive(Debug, Clone)]
pub struct CodeToEditSymbolResponse {
    code_to_edit_list: CodeToEditList,
    code_to_not_edit_list: CodeToNotEditList,
}

impl CodeToEditSymbolResponse {
    pub fn new(
        code_to_edit_list: CodeToEditList,
        code_to_not_edit_list: CodeToNotEditList,
    ) -> Self {
        Self {
            code_to_edit_list,
            code_to_not_edit_list,
        }
    }

    pub fn code_to_edit_list(&self) -> &CodeToEditList {
        &self.code_to_edit_list
    }

    pub fn code_to_not_edit_list(&self) -> &CodeToNotEditList {
        &self.code_to_not_edit_list
    }

    fn unescape_xml(s: String) -> String {
        s.replace("\"", "&quot;")
            .replace("'", "&apos;")
            .replace(">", "&gt;")
            .replace("<", "&lt;")
            .replace("&", "&amp;")
    }

    fn parse_response_section(response: &str) -> String {
        let mut is_inside_reason = false;
        let mut new_lines = vec![];
        for line in response.lines() {
            if line == "<reason_to_edit>"
                || line == "<reason_to_not_edit>"
                || line == "<reason_to_probe>"
                || line == "<reason_to_not_probe>"
            {
                is_inside_reason = true;
                new_lines.push(line.to_owned());
                continue;
            } else if line == "</reason_to_edit>"
                || line == "</reason_to_not_edit>"
                || line == "</reason_to_probe>"
                || line == "</reason_to_not_probe>"
            {
                is_inside_reason = false;
                new_lines.push(line.to_owned());
                continue;
            }
            if is_inside_reason {
                new_lines.push(CodeToEditSymbolResponse::unescape_xml(line.to_owned()));
            } else {
                new_lines.push(line.to_owned());
            }
        }
        new_lines.join("\n")
    }

    pub fn parse_response(response: &str) -> Result<Self, CodeToEditFilteringError> {
        // first we want to find the code_to_edit and code_to_not_edit sections
        let mut code_to_edit_list = response
            .lines()
            .into_iter()
            .skip_while(|line| !line.contains("<code_to_edit_list>"))
            .skip(1)
            .take_while(|line| !line.contains("</code_to_edit_list>"))
            .collect::<Vec<&str>>()
            .join("\n");
        let mut code_to_not_edit_list = response
            .lines()
            .into_iter()
            .skip_while(|line| !line.contains("<code_to_not_edit_list>"))
            .skip(1)
            .take_while(|line| !line.contains("</code_to_not_edit_list>"))
            .collect::<Vec<&str>>()
            .join("\n");
        code_to_edit_list = format!(
            "<code_to_edit_list>
{code_to_edit_list}
</code_to_edit_list>"
        );
        code_to_not_edit_list = format!(
            "<code_to_not_edit_list>
{code_to_not_edit_list}
</code_to_not_edit_list>"
        );
        code_to_edit_list = CodeToEditSymbolResponse::parse_response_section(&code_to_edit_list);
        code_to_not_edit_list =
            CodeToEditSymbolResponse::parse_response_section(&code_to_not_edit_list);
        println!("code_to_edit_list:\n{}", &code_to_edit_list);
        println!("code_to_not_edit_list:\n{}", &code_to_not_edit_list);
        let code_to_edit_list = from_str::<CodeToEditList>(&code_to_edit_list)
            .map_err(|e| CodeToEditFilteringError::SerdeError(e))?;
        let code_to_not_edit_list = from_str::<CodeToNotEditList>(&code_to_not_edit_list)
            .map_err(|e| CodeToEditFilteringError::SerdeError(e))?;
        Ok(CodeToEditSymbolResponse::new(
            code_to_edit_list,
            code_to_not_edit_list,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct CodeToProbeSymbolResponse {
    code_to_probe_list: CodeToProbeList,
}

impl CodeToProbeSymbolResponse {
    pub fn new(code_to_probe_list: CodeToProbeList) -> Self {
        Self { code_to_probe_list }
    }

    pub fn code_to_probe_list(&self) -> &CodeToProbeList {
        &self.code_to_probe_list
    }
}

#[derive(Debug, Clone)]
pub struct CodeToProbeFilterResponse {
    snippets_to_probe_ordered: Vec<SnippetWithReason>,
    _snippets_to_not_probe: Vec<SnippetWithReason>,
}

impl CodeToProbeFilterResponse {
    pub fn new(
        snippets_to_probe_ordered: Vec<SnippetWithReason>,
        snippets_to_not_probe: Vec<SnippetWithReason>,
    ) -> Self {
        Self {
            _snippets_to_not_probe: snippets_to_not_probe,
            snippets_to_probe_ordered,
        }
    }

    pub fn snippets_to_probe_ordered(self) -> Vec<SnippetWithReason> {
        self.snippets_to_probe_ordered
    }
}

#[derive(Debug, Clone)]
pub struct CodeToEditFilterResponse {
    _snippets_to_edit_ordered: Vec<SnippetWithReason>,
    _snippets_to_not_edit: Vec<SnippetWithReason>,
}

impl CodeToEditFilterResponse {
    pub fn new(
        snippets_to_edit: Vec<SnippetWithReason>,
        snippets_to_not_edit: Vec<SnippetWithReason>,
    ) -> Self {
        Self {
            _snippets_to_edit_ordered: snippets_to_edit,
            _snippets_to_not_edit: snippets_to_not_edit,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CodeToEditFilterRequest {
    snippets: Vec<Snippet>,
    query: String,
    llm: LLMType,
    provider: LLMProvider,
    api_key: LLMProviderAPIKeys,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CodeToEditSymbolRequest {
    xml_symbol: String,
    query: String,
    llm: LLMType,
    provider: LLMProvider,
    api_key: LLMProviderAPIKeys,
}

impl CodeToEditSymbolRequest {
    pub fn new(
        xml_symbol: String,
        query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_key: LLMProviderAPIKeys,
    ) -> Self {
        Self {
            xml_symbol,
            query,
            llm,
            api_key,
            provider,
        }
    }

    pub fn xml_string(self) -> String {
        self.xml_symbol
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn llm(&self) -> &LLMType {
        &self.llm
    }

    pub fn provider(&self) -> &LLMProvider {
        &self.provider
    }

    pub fn api_key(&self) -> &LLMProviderAPIKeys {
        &self.api_key
    }
}

impl CodeToEditFilterRequest {
    pub fn new(
        snippets: Vec<Snippet>,
        query: String,
        llm: LLMType,
        provider: LLMProvider,
        api_key: LLMProviderAPIKeys,
    ) -> Self {
        Self {
            snippets,
            query,
            llm,
            provider,
            api_key,
        }
    }

    pub fn get_snippets(&self) -> &[Snippet] {
        &self.snippets
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn llm(&self) -> &LLMType {
        &self.llm
    }

    pub fn provider(&self) -> &LLMProvider {
        &self.provider
    }

    pub fn api_key(&self) -> &LLMProviderAPIKeys {
        &self.api_key
    }
}

pub struct CodeToEditFormatterBroker {
    pub llms: HashMap<LLMType, Box<dyn CodeToEditFilterFormatter + Send + Sync>>,
}

impl CodeToEditFormatterBroker {
    pub fn new(llm_broker: Arc<LLMBroker>) -> Self {
        let mut llms: HashMap<LLMType, Box<dyn CodeToEditFilterFormatter + Send + Sync>> =
            Default::default();
        llms.insert(
            LLMType::ClaudeHaiku,
            Box::new(AnthropicCodeToEditFormatter::new(llm_broker.clone())),
        );
        llms.insert(
            LLMType::ClaudeSonnet,
            Box::new(AnthropicCodeToEditFormatter::new(llm_broker.clone())),
        );
        llms.insert(
            LLMType::ClaudeOpus,
            Box::new(AnthropicCodeToEditFormatter::new(llm_broker.clone())),
        );
        llms.insert(
            LLMType::GeminiPro,
            Box::new(AnthropicCodeToEditFormatter::new(llm_broker.clone())),
        );
        llms.insert(
            LLMType::GeminiProFlash,
            Box::new(AnthropicCodeToEditFormatter::new(llm_broker)),
        );
        Self { llms }
    }
}

#[async_trait]
pub trait CodeToEditFilterFormatter {
    async fn filter_code_snippets(
        &self,
        request: CodeToEditFilterRequest,
    ) -> Result<CodeToEditFilterResponse, CodeToEditFilteringError>;

    // TODO(skcd): We need to figure out which symbols we need to keep
    async fn filter_code_snippets_inside_symbol(
        &self,
        request: CodeToEditSymbolRequest,
    ) -> Result<CodeToEditSymbolResponse, CodeToEditFilteringError>;

    // this request is for probing
    async fn filter_code_snippet_inside_symbol_for_probing(
        &self,
        request: CodeToEditFilterRequest,
    ) -> Result<CodeToProbeFilterResponse, CodeToEditFilteringError>;
}

#[async_trait]
impl Tool for CodeToEditFormatterBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        if input.is_probe_subsymbol() {
            let context = input.probe_subsymbol()?;
            if let Some(llm) = self.llms.get(context.llm()) {
                return llm
                    .filter_code_snippet_inside_symbol_for_probing(context)
                    .await
                    .map_err(|e| ToolError::CodeToEditFiltering(e))
                    .map(|response| ToolOutput::probe_sub_symbol(response));
            } else {
                Err(ToolError::WrongToolInput(ToolType::ProbeSubSymbol))
            }
        } else {
            let context = input.filter_code_snippets_request()?;
            match context {
                either::Left(request) => {
                    if let Some(llm) = self.llms.get(&request.llm) {
                        llm.filter_code_snippets(request)
                            .await
                            .map_err(|e| ToolError::CodeToEditFiltering(e))
                            .map(|result| ToolOutput::CodeToEditSnippets(result))
                    } else {
                        Err(ToolError::WrongToolInput(
                            ToolType::FilterCodeSnippetsForEditing,
                        ))
                    }
                }
                either::Right(context) => {
                    if let Some(llm) = self.llms.get(&context.llm) {
                        llm.filter_code_snippets_inside_symbol(context)
                            .await
                            .map_err(|e| ToolError::CodeToEditFiltering(e))
                            .map(|result| ToolOutput::CodeToEditSingleSymbolSnippets(result))
                    } else {
                        Err(ToolError::WrongToolInput(
                            ToolType::FilterCodeSnippetsSingleSymbolForEditing,
                        ))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AnthropicCodeToEditFormatter, CodeToEditSymbolResponse};

    #[test]
    fn test_code_to_edit_list_filtering() {
        let response = format!(r#"
<code_to_edit_list>
<code_to_edit>
<id>0</id>
<reason_to_edit>
This code defines the Collector class, which is responsible for collecting and deleting related objects when an object is deleted. It handles cascading deletes, pre/post delete signals, and optimizations for fast deletes. This is a crucial part of Django's object deletion mechanism, so it's important to understand and potentially debug issues related to deleting objects and their related data.
</reason_to_edit>
</code_to_edit>
</code_to_edit_list>

<code_to_not_edit_list>
<code_to_not_edit>
<id>1</id>
<reason_to_not_edit>
This is the same code as the previous entry, so there's no need to edit it again.
</reason_to_not_edit>
</code_to_not_edit>
</code_to_not_edit_list>"#).to_owned();
        let code_to_edit_formatter = CodeToEditSymbolResponse::parse_response(&response);
        assert!(code_to_edit_formatter.is_ok());
    }
}
