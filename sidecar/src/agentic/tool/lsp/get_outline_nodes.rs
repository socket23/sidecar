//! The editor provides DocumentSymbols which we can use to map back
//! to the outline nodes which we need over here
//! The clutch is that its not perfect and there are language specific
//! tricks which we need to pull off properly, but before we start doing
//! that we should see how well it works for the languages we are interested in

use async_trait::async_trait;

use crate::agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutlineNodesUsingEditorRequest {
    fs_file_path: String,
    editor_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentSymbolPosition {
    line: usize,
    character: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentSymbolRange {
    start: DocumentSymbolPosition,
    end: DocumentSymbolPosition,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentSymbol {
    /// The name of this symbol.
    pub name: String,

    /// More detail for this symbol, e.g. the signature of a function.
    pub detail: Option<String>,

    /// The kind of this symbol.
    pub kind: usize,

    /// The range enclosing this symbol not including leading/trailing whitespace
    /// but everything else, e.g. comments and code.
    pub range: [DocumentSymbolPosition; 2],

    /// The range that should be selected and reveal when this symbol is being picked,
    /// e.g. the name of a function. Must be contained by the `range`.
    #[serde(rename = "selectionRange")]
    pub selection_range: [DocumentSymbolPosition; 2],

    /// Children of this symbol, e.g. properties of a class.
    #[serde(default)]
    pub children: Vec<DocumentSymbol>,
}

pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl SymbolKind {
    /// Convert a usize to SymbolKind
    pub fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(Self::File),
            1 => Some(Self::Module),
            2 => Some(Self::Namespace),
            3 => Some(Self::Package),
            4 => Some(Self::Class),
            5 => Some(Self::Method),
            6 => Some(Self::Property),
            7 => Some(Self::Field),
            8 => Some(Self::Constructor),
            9 => Some(Self::Enum),
            10 => Some(Self::Interface),
            11 => Some(Self::Function),
            12 => Some(Self::Variable),
            13 => Some(Self::Constant),
            14 => Some(Self::String),
            15 => Some(Self::Number),
            16 => Some(Self::Boolean),
            17 => Some(Self::Array),
            18 => Some(Self::Object),
            19 => Some(Self::Key),
            20 => Some(Self::Null),
            21 => Some(Self::EnumMember),
            22 => Some(Self::Struct),
            23 => Some(Self::Event),
            24 => Some(Self::Operator),
            25 => Some(Self::TypeParameter),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutlineNodesUsingEditorResponse {
    // we have to create the outline nodes over here
    outline_nodes: Vec<DocumentSymbol>,
}

impl OutlineNodesUsingEditorRequest {
    pub fn new(fs_file_path: String, editor_url: String) -> Self {
        Self {
            fs_file_path,
            editor_url,
        }
    }
}

pub struct OutlineNodesUsingEditorClient {
    client: reqwest::Client,
}

impl OutlineNodesUsingEditorClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for OutlineNodesUsingEditorClient {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.should_outline_nodes_using_editor()?;
        let editor_endpoint = context.editor_url.to_owned() + "/get_outline_nodes";
        let response = self
            .client
            .post(editor_endpoint)
            .body(serde_json::to_string(&context).map_err(|_e| ToolError::SerdeConversionFailed)?)
            .send()
            .await
            .map_err(|_e| ToolError::ErrorCommunicatingWithEditor)?;
        let response: OutlineNodesUsingEditorResponse = response.json().await.map_err(|e| {
            eprintln!("{:?}", e);
            ToolError::SerdeConversionFailed
        })?;

        Ok(ToolOutput::outline_nodes_using_editor(response))
    }
}

#[cfg(test)]
mod tests {
    use super::OutlineNodesUsingEditorResponse;

    #[test]
    fn test_parsing_response_from_editor() {
        let response = r#"{
  "outline_nodes": [
    {
      "name": "main",
      "detail": "fn()",
      "kind": 11,
      "range": [
        {
          "line": 4,
          "character": 0
        },
        {
          "line": 27,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 5,
          "character": 9
        },
        {
          "line": 5,
          "character": 13
        }
      ],
      "children": []
    }
  ]
}"#;
        let parsed_response = serde_json::from_str::<OutlineNodesUsingEditorResponse>(response);
        println!("{:?}", &parsed_response);
        assert!(parsed_response.is_ok());
    }
}
