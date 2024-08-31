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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum SymbolKind {
    /// The `File` symbol kind.
    File = 0,
    /// The `Module` symbol kind.
    Module = 1,
    /// The `Namespace` symbol kind.
    Namespace = 2,
    /// The `Package` symbol kind.
    Package = 3,
    /// The `Class` symbol kind.
    Class = 4,
    /// The `Method` symbol kind.
    Method = 5,
    /// The `Property` symbol kind.
    Property = 6,
    /// The `Field` symbol kind.
    Field = 7,
    /// The `Constructor` symbol kind.
    Constructor = 8,
    /// The `Enum` symbol kind.
    Enum = 9,
    /// The `Interface` symbol kind.
    Interface = 10,
    /// The `Function` symbol kind.
    Function = 11,
    /// The `Variable` symbol kind.
    Variable = 12,
    /// The `Constant` symbol kind.
    Constant = 13,
    /// The `String` symbol kind.
    String = 14,
    /// The `Number` symbol kind.
    Number = 15,
    /// The `Boolean` symbol kind.
    Boolean = 16,
    /// The `Array` symbol kind.
    Array = 17,
    /// The `Object` symbol kind.
    Object = 18,
    /// The `Key` symbol kind.
    Key = 19,
    /// The `Null` symbol kind.
    Null = 20,
    /// The `EnumMember` symbol kind.
    EnumMember = 21,
    /// The `Struct` symbol kind.
    Struct = 22,
    /// The `Event` symbol kind.
    Event = 23,
    /// The `Operator` symbol kind.
    Operator = 24,
    /// The `TypeParameter` symbol kind.
    TypeParameter = 25,
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
