//! The editor provides DocumentSymbols which we can use to map back
//! to the outline nodes which we need over here
//! The clutch is that its not perfect and there are language specific
//! tricks which we need to pull off properly, but before we start doing
//! that we should see how well it works for the languages we are interested in

use async_trait::async_trait;

use crate::{
    agentic::tool::{errors::ToolError, input::ToolInput, output::ToolOutput, r#type::Tool},
    chunking::types::OutlineNode,
};

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
            // in case of module we want to keep going deeper
            // as this is not a top level symbol which we consider
            1 => Some(Self::Module),
            // for namespace as well, we want to keep going deeper
            2 => Some(Self::Namespace),
            // same for package as well, we want to keep going deeper
            3 => Some(Self::Package),
            // class here is the Struct in rust land so we do want to classify
            // this as class declaration
            4 => Some(Self::Class),
            // method can be a function inside class which we want to track
            5 => Some(Self::Method),
            // we can ignore this safely for now
            6 => Some(Self::Property),
            // similarly for this we can ignore this safely
            7 => Some(Self::Field),
            // special but not really, ends up being a function infact in most languages
            8 => Some(Self::Constructor),
            // this gets mapped to the class declaration
            9 => Some(Self::Enum),
            // this should also get mapped to class declaration
            10 => Some(Self::Interface),
            // this can be a global function or a method in a class
            11 => Some(Self::Function),
            // only track if this is global and belongs to a file or a module
            12 => Some(Self::Variable),
            // we want to track this for rust like languages only if this is global
            13 => Some(Self::Constant),
            // ignore for now
            14 => Some(Self::String),
            // ignore for now
            15 => Some(Self::Number),
            // ignore for now
            16 => Some(Self::Boolean),
            // ignore for now
            17 => Some(Self::Array),
            // ignore for now
            18 => Some(Self::Object),
            // ignore for now
            19 => Some(Self::Key),
            // ignore for now
            20 => Some(Self::Null),
            // ignore for now
            21 => Some(Self::EnumMember),
            // this is the impl block in most cases so we can classify this as
            // class instead (and in python/js land, this will be the class itself)
            22 => Some(Self::Struct),
            // ignore for now
            23 => Some(Self::Event),
            // ignore for now
            24 => Some(Self::Operator),
            // ignore for now
            25 => Some(Self::TypeParameter),
            _ => None,
        }
    }
}

/// now we want to convert this back to the OutlineNode types we are interested in
/// from the documentSymbol
/// to go about this the right way we have to go through all the documentSymbols and figure
/// out which one we are talking about and why and translate them properly
fn document_symbols_to_outline_nodes(
    _language: String,
    _file_content: &str,
    _document_symbols: Vec<DocumentSymbol>,
) -> Vec<OutlineNode> {
    // initial mapping done, now we can use AI to generate this part of the code
    // and map things out properly
    vec![]
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutlineNodesUsingEditorResponse {
    file_content: String,
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
  ],
  "file_content": "testing"
}"#;
        let parsed_response = serde_json::from_str::<OutlineNodesUsingEditorResponse>(response);
        println!("{:?}", &parsed_response);
        assert!(parsed_response.is_ok());
    }

    #[test]
    fn test_outline_node_generation() {
        let outline_nodes_from_editor = r#"{
  "outline_nodes": [
    {
      "name": "LLMType",
      "detail": "",
      "kind": 9,
      "range": [
        {
          "line": 11,
          "character": 0
        },
        {
          "line": 66,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 13,
          "character": 9
        },
        {
          "line": 13,
          "character": 16
        }
      ],
      "children": [
        {
          "name": "Mixtral",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 14,
              "character": 4
            },
            {
              "line": 15,
              "character": 11
            }
          ],
          "selectionRange": [
            {
              "line": 15,
              "character": 4
            },
            {
              "line": 15,
              "character": 11
            }
          ],
          "children": []
        },
        {
          "name": "MistralInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 16,
              "character": 4
            },
            {
              "line": 17,
              "character": 19
            }
          ],
          "selectionRange": [
            {
              "line": 17,
              "character": 4
            },
            {
              "line": 17,
              "character": 19
            }
          ],
          "children": []
        },
        {
          "name": "Gpt4",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 18,
              "character": 4
            },
            {
              "line": 19,
              "character": 8
            }
          ],
          "selectionRange": [
            {
              "line": 19,
              "character": 4
            },
            {
              "line": 19,
              "character": 8
            }
          ],
          "children": []
        },
        {
          "name": "GPT3_5_16k",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 20,
              "character": 4
            },
            {
              "line": 21,
              "character": 14
            }
          ],
          "selectionRange": [
            {
              "line": 21,
              "character": 4
            },
            {
              "line": 21,
              "character": 14
            }
          ],
          "children": []
        },
        {
          "name": "Gpt4_32k",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 22,
              "character": 4
            },
            {
              "line": 23,
              "character": 12
            }
          ],
          "selectionRange": [
            {
              "line": 23,
              "character": 4
            },
            {
              "line": 23,
              "character": 12
            }
          ],
          "children": []
        },
        {
          "name": "Gpt4O",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 24,
              "character": 4
            },
            {
              "line": 25,
              "character": 9
            }
          ],
          "selectionRange": [
            {
              "line": 25,
              "character": 4
            },
            {
              "line": 25,
              "character": 9
            }
          ],
          "children": []
        },
        {
          "name": "Gpt4OMini",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 26,
              "character": 4
            },
            {
              "line": 27,
              "character": 13
            }
          ],
          "selectionRange": [
            {
              "line": 27,
              "character": 4
            },
            {
              "line": 27,
              "character": 13
            }
          ],
          "children": []
        },
        {
          "name": "Gpt4Turbo",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 28,
              "character": 4
            },
            {
              "line": 29,
              "character": 13
            }
          ],
          "selectionRange": [
            {
              "line": 29,
              "character": 4
            },
            {
              "line": 29,
              "character": 13
            }
          ],
          "children": []
        },
        {
          "name": "DeepSeekCoder1_3BInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 30,
              "character": 4
            },
            {
              "line": 31,
              "character": 29
            }
          ],
          "selectionRange": [
            {
              "line": 31,
              "character": 4
            },
            {
              "line": 31,
              "character": 29
            }
          ],
          "children": []
        },
        {
          "name": "DeepSeekCoder33BInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 32,
              "character": 4
            },
            {
              "line": 33,
              "character": 28
            }
          ],
          "selectionRange": [
            {
              "line": 33,
              "character": 4
            },
            {
              "line": 33,
              "character": 28
            }
          ],
          "children": []
        },
        {
          "name": "DeepSeekCoder6BInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 34,
              "character": 4
            },
            {
              "line": 35,
              "character": 27
            }
          ],
          "selectionRange": [
            {
              "line": 35,
              "character": 4
            },
            {
              "line": 35,
              "character": 27
            }
          ],
          "children": []
        },
        {
          "name": "DeepSeekCoderV2",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 36,
              "character": 4
            },
            {
              "line": 37,
              "character": 19
            }
          ],
          "selectionRange": [
            {
              "line": 37,
              "character": 4
            },
            {
              "line": 37,
              "character": 19
            }
          ],
          "children": []
        },
        {
          "name": "CodeLLama70BInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 38,
              "character": 4
            },
            {
              "line": 39,
              "character": 24
            }
          ],
          "selectionRange": [
            {
              "line": 39,
              "character": 4
            },
            {
              "line": 39,
              "character": 24
            }
          ],
          "children": []
        },
        {
          "name": "CodeLlama13BInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 40,
              "character": 4
            },
            {
              "line": 41,
              "character": 24
            }
          ],
          "selectionRange": [
            {
              "line": 41,
              "character": 4
            },
            {
              "line": 41,
              "character": 24
            }
          ],
          "children": []
        },
        {
          "name": "CodeLlama7BInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 42,
              "character": 4
            },
            {
              "line": 43,
              "character": 23
            }
          ],
          "selectionRange": [
            {
              "line": 43,
              "character": 4
            },
            {
              "line": 43,
              "character": 23
            }
          ],
          "children": []
        },
        {
          "name": "Llama3_8bInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 44,
              "character": 4
            },
            {
              "line": 45,
              "character": 21
            }
          ],
          "selectionRange": [
            {
              "line": 45,
              "character": 4
            },
            {
              "line": 45,
              "character": 21
            }
          ],
          "children": []
        },
        {
          "name": "Llama3_1_8bInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 46,
              "character": 4
            },
            {
              "line": 47,
              "character": 23
            }
          ],
          "selectionRange": [
            {
              "line": 47,
              "character": 4
            },
            {
              "line": 47,
              "character": 23
            }
          ],
          "children": []
        },
        {
          "name": "Llama3_1_70bInstruct",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 48,
              "character": 4
            },
            {
              "line": 49,
              "character": 24
            }
          ],
          "selectionRange": [
            {
              "line": 49,
              "character": 4
            },
            {
              "line": 49,
              "character": 24
            }
          ],
          "children": []
        },
        {
          "name": "ClaudeOpus",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 50,
              "character": 4
            },
            {
              "line": 51,
              "character": 14
            }
          ],
          "selectionRange": [
            {
              "line": 51,
              "character": 4
            },
            {
              "line": 51,
              "character": 14
            }
          ],
          "children": []
        },
        {
          "name": "ClaudeSonnet",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 52,
              "character": 4
            },
            {
              "line": 53,
              "character": 16
            }
          ],
          "selectionRange": [
            {
              "line": 53,
              "character": 4
            },
            {
              "line": 53,
              "character": 16
            }
          ],
          "children": []
        },
        {
          "name": "ClaudeHaiku",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 54,
              "character": 4
            },
            {
              "line": 55,
              "character": 15
            }
          ],
          "selectionRange": [
            {
              "line": 55,
              "character": 4
            },
            {
              "line": 55,
              "character": 15
            }
          ],
          "children": []
        },
        {
          "name": "PPLXSonnetSmall",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 56,
              "character": 4
            },
            {
              "line": 57,
              "character": 19
            }
          ],
          "selectionRange": [
            {
              "line": 57,
              "character": 4
            },
            {
              "line": 57,
              "character": 19
            }
          ],
          "children": []
        },
        {
          "name": "CohereRerankV3",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 58,
              "character": 4
            },
            {
              "line": 59,
              "character": 18
            }
          ],
          "selectionRange": [
            {
              "line": 59,
              "character": 4
            },
            {
              "line": 59,
              "character": 18
            }
          ],
          "children": []
        },
        {
          "name": "GeminiPro",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 60,
              "character": 4
            },
            {
              "line": 61,
              "character": 13
            }
          ],
          "selectionRange": [
            {
              "line": 61,
              "character": 4
            },
            {
              "line": 61,
              "character": 13
            }
          ],
          "children": []
        },
        {
          "name": "GeminiProFlash",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 62,
              "character": 4
            },
            {
              "line": 63,
              "character": 18
            }
          ],
          "selectionRange": [
            {
              "line": 63,
              "character": 4
            },
            {
              "line": 63,
              "character": 18
            }
          ],
          "children": []
        },
        {
          "name": "Custom",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 64,
              "character": 4
            },
            {
              "line": 65,
              "character": 18
            }
          ],
          "selectionRange": [
            {
              "line": 65,
              "character": 4
            },
            {
              "line": 65,
              "character": 10
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl Serialize for LLMType",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 68,
          "character": 0
        },
        {
          "line": 78,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 68,
          "character": 19
        },
        {
          "line": 68,
          "character": 26
        }
      ],
      "children": [
        {
          "name": "serialize",
          "detail": "fn<S>(&self, serializer: S) -> Result<S::Ok, S::Error>",
          "kind": 11,
          "range": [
            {
              "line": 69,
              "character": 4
            },
            {
              "line": 77,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 69,
              "character": 7
            },
            {
              "line": 69,
              "character": 16
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl Deserialize<'de> for LLMType",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 80,
          "character": 0
        },
        {
          "line": 131,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 80,
          "character": 31
        },
        {
          "line": 80,
          "character": 38
        }
      ],
      "children": [
        {
          "name": "deserialize",
          "detail": "fn<D>(deserializer: D) -> Result<Self, D::Error>",
          "kind": 11,
          "range": [
            {
              "line": 81,
              "character": 4
            },
            {
              "line": 130,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 81,
              "character": 7
            },
            {
              "line": 81,
              "character": 18
            }
          ],
          "children": [
            {
              "name": "LLMTypeVisitor",
              "detail": "",
              "kind": 22,
              "range": [
                {
                  "line": 85,
                  "character": 8
                },
                {
                  "line": 85,
                  "character": 30
                }
              ],
              "selectionRange": [
                {
                  "line": 85,
                  "character": 15
                },
                {
                  "line": 85,
                  "character": 29
                }
              ],
              "children": []
            },
            {
              "name": "impl Visitor<'de> for LLMTypeVisitor",
              "detail": "",
              "kind": 18,
              "range": [
                {
                  "line": 87,
                  "character": 8
                },
                {
                  "line": 127,
                  "character": 9
                }
              ],
              "selectionRange": [
                {
                  "line": 87,
                  "character": 35
                },
                {
                  "line": 87,
                  "character": 49
                }
              ],
              "children": [
                {
                  "name": "Value",
                  "detail": "LLMType",
                  "kind": 25,
                  "range": [
                    {
                      "line": 88,
                      "character": 12
                    },
                    {
                      "line": 88,
                      "character": 33
                    }
                  ],
                  "selectionRange": [
                    {
                      "line": 88,
                      "character": 17
                    },
                    {
                      "line": 88,
                      "character": 22
                    }
                  ],
                  "children": []
                },
                {
                  "name": "expecting",
                  "detail": "fn(&self, formatter: &mut fmt::Formatter) -> fmt::Result",
                  "kind": 11,
                  "range": [
                    {
                      "line": 90,
                      "character": 12
                    },
                    {
                      "line": 92,
                      "character": 13
                    }
                  ],
                  "selectionRange": [
                    {
                      "line": 90,
                      "character": 15
                    },
                    {
                      "line": 90,
                      "character": 24
                    }
                  ],
                  "children": []
                },
                {
                  "name": "visit_str",
                  "detail": "fn<E>(self, value: &str) -> Result<LLMType, E>",
                  "kind": 11,
                  "range": [
                    {
                      "line": 94,
                      "character": 12
                    },
                    {
                      "line": 126,
                      "character": 13
                    }
                  ],
                  "selectionRange": [
                    {
                      "line": 94,
                      "character": 15
                    },
                    {
                      "line": 94,
                      "character": 24
                    }
                  ],
                  "children": []
                }
              ]
            }
          ]
        }
      ]
    },
    {
      "name": "impl LLMType",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 133,
          "character": 0
        },
        {
          "line": 177,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 133,
          "character": 5
        },
        {
          "line": 133,
          "character": 12
        }
      ],
      "children": [
        {
          "name": "is_openai",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 134,
              "character": 4
            },
            {
              "line": 144,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 134,
              "character": 11
            },
            {
              "line": 134,
              "character": 20
            }
          ],
          "children": []
        },
        {
          "name": "is_custom",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 146,
              "character": 4
            },
            {
              "line": 148,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 146,
              "character": 11
            },
            {
              "line": 146,
              "character": 20
            }
          ],
          "children": []
        },
        {
          "name": "is_anthropic",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 150,
              "character": 4
            },
            {
              "line": 155,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 150,
              "character": 11
            },
            {
              "line": 150,
              "character": 23
            }
          ],
          "children": []
        },
        {
          "name": "is_openai_gpt4o",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 157,
              "character": 4
            },
            {
              "line": 159,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 157,
              "character": 11
            },
            {
              "line": 157,
              "character": 26
            }
          ],
          "children": []
        },
        {
          "name": "is_gemini_model",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 161,
              "character": 4
            },
            {
              "line": 163,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 161,
              "character": 11
            },
            {
              "line": 161,
              "character": 26
            }
          ],
          "children": []
        },
        {
          "name": "is_gemini_pro",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 165,
              "character": 4
            },
            {
              "line": 167,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 165,
              "character": 11
            },
            {
              "line": 165,
              "character": 24
            }
          ],
          "children": []
        },
        {
          "name": "is_togetherai_model",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 169,
              "character": 4
            },
            {
              "line": 176,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 169,
              "character": 11
            },
            {
              "line": 169,
              "character": 30
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl fmt::Display for LLMType",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 179,
          "character": 0
        },
        {
          "line": 210,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 179,
          "character": 22
        },
        {
          "line": 179,
          "character": 29
        }
      ],
      "children": [
        {
          "name": "fmt",
          "detail": "fn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result",
          "kind": 11,
          "range": [
            {
              "line": 180,
              "character": 4
            },
            {
              "line": 209,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 180,
              "character": 7
            },
            {
              "line": 180,
              "character": 10
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "LLMClientRole",
      "detail": "",
      "kind": 9,
      "range": [
        {
          "line": 212,
          "character": 0
        },
        {
          "line": 221,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 213,
          "character": 9
        },
        {
          "line": 213,
          "character": 22
        }
      ],
      "children": [
        {
          "name": "System",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 214,
              "character": 4
            },
            {
              "line": 214,
              "character": 10
            }
          ],
          "selectionRange": [
            {
              "line": 214,
              "character": 4
            },
            {
              "line": 214,
              "character": 10
            }
          ],
          "children": []
        },
        {
          "name": "User",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 215,
              "character": 4
            },
            {
              "line": 215,
              "character": 8
            }
          ],
          "selectionRange": [
            {
              "line": 215,
              "character": 4
            },
            {
              "line": 215,
              "character": 8
            }
          ],
          "children": []
        },
        {
          "name": "Assistant",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 216,
              "character": 4
            },
            {
              "line": 216,
              "character": 13
            }
          ],
          "selectionRange": [
            {
              "line": 216,
              "character": 4
            },
            {
              "line": 216,
              "character": 13
            }
          ],
          "children": []
        },
        {
          "name": "Function",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 217,
              "character": 4
            },
            {
              "line": 220,
              "character": 12
            }
          ],
          "selectionRange": [
            {
              "line": 220,
              "character": 4
            },
            {
              "line": 220,
              "character": 12
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl LLMClientRole",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 223,
          "character": 0
        },
        {
          "line": 248,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 223,
          "character": 5
        },
        {
          "line": 223,
          "character": 18
        }
      ],
      "children": [
        {
          "name": "is_system",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 224,
              "character": 4
            },
            {
              "line": 226,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 224,
              "character": 11
            },
            {
              "line": 224,
              "character": 20
            }
          ],
          "children": []
        },
        {
          "name": "is_user",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 228,
              "character": 4
            },
            {
              "line": 230,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 228,
              "character": 11
            },
            {
              "line": 228,
              "character": 18
            }
          ],
          "children": []
        },
        {
          "name": "is_assistant",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 232,
              "character": 4
            },
            {
              "line": 234,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 232,
              "character": 11
            },
            {
              "line": 232,
              "character": 23
            }
          ],
          "children": []
        },
        {
          "name": "is_function",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 236,
              "character": 4
            },
            {
              "line": 238,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 236,
              "character": 11
            },
            {
              "line": 236,
              "character": 22
            }
          ],
          "children": []
        },
        {
          "name": "to_string",
          "detail": "fn(&self) -> String",
          "kind": 11,
          "range": [
            {
              "line": 240,
              "character": 4
            },
            {
              "line": 247,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 240,
              "character": 11
            },
            {
              "line": 240,
              "character": 20
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "LLMClientMessageFunctionCall",
      "detail": "",
      "kind": 22,
      "range": [
        {
          "line": 250,
          "character": 0
        },
        {
          "line": 256,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 251,
          "character": 11
        },
        {
          "line": 251,
          "character": 39
        }
      ],
      "children": [
        {
          "name": "name",
          "detail": "String",
          "kind": 7,
          "range": [
            {
              "line": 252,
              "character": 4
            },
            {
              "line": 252,
              "character": 16
            }
          ],
          "selectionRange": [
            {
              "line": 252,
              "character": 4
            },
            {
              "line": 252,
              "character": 8
            }
          ],
          "children": []
        },
        {
          "name": "arguments",
          "detail": "String",
          "kind": 7,
          "range": [
            {
              "line": 253,
              "character": 4
            },
            {
              "line": 255,
              "character": 21
            }
          ],
          "selectionRange": [
            {
              "line": 255,
              "character": 4
            },
            {
              "line": 255,
              "character": 13
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl LLMClientMessageFunctionCall",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 258,
          "character": 0
        },
        {
          "line": 266,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 258,
          "character": 5
        },
        {
          "line": 258,
          "character": 33
        }
      ],
      "children": [
        {
          "name": "name",
          "detail": "fn(&self) -> &str",
          "kind": 11,
          "range": [
            {
              "line": 259,
              "character": 4
            },
            {
              "line": 261,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 259,
              "character": 11
            },
            {
              "line": 259,
              "character": 15
            }
          ],
          "children": []
        },
        {
          "name": "arguments",
          "detail": "fn(&self) -> &str",
          "kind": 11,
          "range": [
            {
              "line": 263,
              "character": 4
            },
            {
              "line": 265,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 263,
              "character": 11
            },
            {
              "line": 263,
              "character": 20
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "LLMClientMessageFunctionReturn",
      "detail": "",
      "kind": 22,
      "range": [
        {
          "line": 268,
          "character": 0
        },
        {
          "line": 272,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 269,
          "character": 11
        },
        {
          "line": 269,
          "character": 41
        }
      ],
      "children": [
        {
          "name": "name",
          "detail": "String",
          "kind": 7,
          "range": [
            {
              "line": 270,
              "character": 4
            },
            {
              "line": 270,
              "character": 16
            }
          ],
          "selectionRange": [
            {
              "line": 270,
              "character": 4
            },
            {
              "line": 270,
              "character": 8
            }
          ],
          "children": []
        },
        {
          "name": "content",
          "detail": "String",
          "kind": 7,
          "range": [
            {
              "line": 271,
              "character": 4
            },
            {
              "line": 271,
              "character": 19
            }
          ],
          "selectionRange": [
            {
              "line": 271,
              "character": 4
            },
            {
              "line": 271,
              "character": 11
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl LLMClientMessageFunctionReturn",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 274,
          "character": 0
        },
        {
          "line": 282,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 274,
          "character": 5
        },
        {
          "line": 274,
          "character": 35
        }
      ],
      "children": [
        {
          "name": "name",
          "detail": "fn(&self) -> &str",
          "kind": 11,
          "range": [
            {
              "line": 275,
              "character": 4
            },
            {
              "line": 277,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 275,
              "character": 11
            },
            {
              "line": 275,
              "character": 15
            }
          ],
          "children": []
        },
        {
          "name": "content",
          "detail": "fn(&self) -> &str",
          "kind": 11,
          "range": [
            {
              "line": 279,
              "character": 4
            },
            {
              "line": 281,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 279,
              "character": 11
            },
            {
              "line": 279,
              "character": 18
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "LLMClientMessage",
      "detail": "",
      "kind": 22,
      "range": [
        {
          "line": 284,
          "character": 0
        },
        {
          "line": 292,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 285,
          "character": 11
        },
        {
          "line": 285,
          "character": 27
        }
      ],
      "children": [
        {
          "name": "role",
          "detail": "LLMClientRole",
          "kind": 7,
          "range": [
            {
              "line": 286,
              "character": 4
            },
            {
              "line": 286,
              "character": 23
            }
          ],
          "selectionRange": [
            {
              "line": 286,
              "character": 4
            },
            {
              "line": 286,
              "character": 8
            }
          ],
          "children": []
        },
        {
          "name": "message",
          "detail": "String",
          "kind": 7,
          "range": [
            {
              "line": 287,
              "character": 4
            },
            {
              "line": 287,
              "character": 19
            }
          ],
          "selectionRange": [
            {
              "line": 287,
              "character": 4
            },
            {
              "line": 287,
              "character": 11
            }
          ],
          "children": []
        },
        {
          "name": "function_call",
          "detail": "Option<LLMClientMessageFunctionCall>",
          "kind": 7,
          "range": [
            {
              "line": 288,
              "character": 4
            },
            {
              "line": 288,
              "character": 55
            }
          ],
          "selectionRange": [
            {
              "line": 288,
              "character": 4
            },
            {
              "line": 288,
              "character": 17
            }
          ],
          "children": []
        },
        {
          "name": "function_return",
          "detail": "Option<LLMClientMessageFunctionReturn>",
          "kind": 7,
          "range": [
            {
              "line": 289,
              "character": 4
            },
            {
              "line": 289,
              "character": 59
            }
          ],
          "selectionRange": [
            {
              "line": 289,
              "character": 4
            },
            {
              "line": 289,
              "character": 19
            }
          ],
          "children": []
        },
        {
          "name": "cache_point",
          "detail": "bool",
          "kind": 7,
          "range": [
            {
              "line": 290,
              "character": 4
            },
            {
              "line": 291,
              "character": 21
            }
          ],
          "selectionRange": [
            {
              "line": 291,
              "character": 4
            },
            {
              "line": 291,
              "character": 15
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl LLMClientMessage",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 294,
          "character": 0
        },
        {
          "line": 393,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 294,
          "character": 5
        },
        {
          "line": 294,
          "character": 21
        }
      ],
      "children": [
        {
          "name": "new",
          "detail": "fn(role: LLMClientRole, message: String) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 295,
              "character": 4
            },
            {
              "line": 303,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 295,
              "character": 11
            },
            {
              "line": 295,
              "character": 14
            }
          ],
          "children": []
        },
        {
          "name": "concat_message",
          "detail": "fn(&mut self, message: &str)",
          "kind": 11,
          "range": [
            {
              "line": 305,
              "character": 4
            },
            {
              "line": 307,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 305,
              "character": 11
            },
            {
              "line": 305,
              "character": 25
            }
          ],
          "children": []
        },
        {
          "name": "concat",
          "detail": "fn(self, other: Self) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 309,
              "character": 4
            },
            {
              "line": 325,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 309,
              "character": 11
            },
            {
              "line": 309,
              "character": 17
            }
          ],
          "children": []
        },
        {
          "name": "function_call",
          "detail": "fn(name: String, arguments: String) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 327,
              "character": 4
            },
            {
              "line": 335,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 327,
              "character": 11
            },
            {
              "line": 327,
              "character": 24
            }
          ],
          "children": []
        },
        {
          "name": "function_return",
          "detail": "fn(name: String, content: String) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 337,
              "character": 4
            },
            {
              "line": 345,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 337,
              "character": 11
            },
            {
              "line": 337,
              "character": 26
            }
          ],
          "children": []
        },
        {
          "name": "user",
          "detail": "fn(message: String) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 347,
              "character": 4
            },
            {
              "line": 349,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 347,
              "character": 11
            },
            {
              "line": 347,
              "character": 15
            }
          ],
          "children": []
        },
        {
          "name": "assistant",
          "detail": "fn(message: String) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 351,
              "character": 4
            },
            {
              "line": 353,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 351,
              "character": 11
            },
            {
              "line": 351,
              "character": 20
            }
          ],
          "children": []
        },
        {
          "name": "system",
          "detail": "fn(message: String) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 355,
              "character": 4
            },
            {
              "line": 357,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 355,
              "character": 11
            },
            {
              "line": 355,
              "character": 17
            }
          ],
          "children": []
        },
        {
          "name": "content",
          "detail": "fn(&self) -> &str",
          "kind": 11,
          "range": [
            {
              "line": 359,
              "character": 4
            },
            {
              "line": 361,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 359,
              "character": 11
            },
            {
              "line": 359,
              "character": 18
            }
          ],
          "children": []
        },
        {
          "name": "set_empty_content",
          "detail": "fn(&mut self)",
          "kind": 11,
          "range": [
            {
              "line": 363,
              "character": 4
            },
            {
              "line": 367,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 363,
              "character": 11
            },
            {
              "line": 363,
              "character": 28
            }
          ],
          "children": []
        },
        {
          "name": "function",
          "detail": "fn(message: String) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 369,
              "character": 4
            },
            {
              "line": 371,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 369,
              "character": 11
            },
            {
              "line": 369,
              "character": 19
            }
          ],
          "children": []
        },
        {
          "name": "role",
          "detail": "fn(&self) -> &LLMClientRole",
          "kind": 11,
          "range": [
            {
              "line": 373,
              "character": 4
            },
            {
              "line": 375,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 373,
              "character": 11
            },
            {
              "line": 373,
              "character": 15
            }
          ],
          "children": []
        },
        {
          "name": "get_function_call",
          "detail": "fn(&self) -> Option<&LLMClientMessageFunctionCall>",
          "kind": 11,
          "range": [
            {
              "line": 377,
              "character": 4
            },
            {
              "line": 379,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 377,
              "character": 11
            },
            {
              "line": 377,
              "character": 28
            }
          ],
          "children": []
        },
        {
          "name": "get_function_return",
          "detail": "fn(&self) -> Option<&LLMClientMessageFunctionReturn>",
          "kind": 11,
          "range": [
            {
              "line": 381,
              "character": 4
            },
            {
              "line": 383,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 381,
              "character": 11
            },
            {
              "line": 381,
              "character": 30
            }
          ],
          "children": []
        },
        {
          "name": "cache_point",
          "detail": "fn(mut self) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 385,
              "character": 4
            },
            {
              "line": 388,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 385,
              "character": 11
            },
            {
              "line": 385,
              "character": 22
            }
          ],
          "children": []
        },
        {
          "name": "is_cache_point",
          "detail": "fn(&self) -> bool",
          "kind": 11,
          "range": [
            {
              "line": 390,
              "character": 4
            },
            {
              "line": 392,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 390,
              "character": 11
            },
            {
              "line": 390,
              "character": 25
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "LLMClientCompletionRequest",
      "detail": "",
      "kind": 22,
      "range": [
        {
          "line": 395,
          "character": 0
        },
        {
          "line": 403,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 396,
          "character": 11
        },
        {
          "line": 396,
          "character": 37
        }
      ],
      "children": [
        {
          "name": "model",
          "detail": "LLMType",
          "kind": 7,
          "range": [
            {
              "line": 397,
              "character": 4
            },
            {
              "line": 397,
              "character": 18
            }
          ],
          "selectionRange": [
            {
              "line": 397,
              "character": 4
            },
            {
              "line": 397,
              "character": 9
            }
          ],
          "children": []
        },
        {
          "name": "messages",
          "detail": "Vec<LLMClientMessage>",
          "kind": 7,
          "range": [
            {
              "line": 398,
              "character": 4
            },
            {
              "line": 398,
              "character": 35
            }
          ],
          "selectionRange": [
            {
              "line": 398,
              "character": 4
            },
            {
              "line": 398,
              "character": 12
            }
          ],
          "children": []
        },
        {
          "name": "temperature",
          "detail": "f32",
          "kind": 7,
          "range": [
            {
              "line": 399,
              "character": 4
            },
            {
              "line": 399,
              "character": 20
            }
          ],
          "selectionRange": [
            {
              "line": 399,
              "character": 4
            },
            {
              "line": 399,
              "character": 15
            }
          ],
          "children": []
        },
        {
          "name": "frequency_penalty",
          "detail": "Option<f32>",
          "kind": 7,
          "range": [
            {
              "line": 400,
              "character": 4
            },
            {
              "line": 400,
              "character": 34
            }
          ],
          "selectionRange": [
            {
              "line": 400,
              "character": 4
            },
            {
              "line": 400,
              "character": 21
            }
          ],
          "children": []
        },
        {
          "name": "stop_words",
          "detail": "Option<Vec<String>>",
          "kind": 7,
          "range": [
            {
              "line": 401,
              "character": 4
            },
            {
              "line": 401,
              "character": 35
            }
          ],
          "selectionRange": [
            {
              "line": 401,
              "character": 4
            },
            {
              "line": 401,
              "character": 14
            }
          ],
          "children": []
        },
        {
          "name": "max_tokens",
          "detail": "Option<usize>",
          "kind": 7,
          "range": [
            {
              "line": 402,
              "character": 4
            },
            {
              "line": 402,
              "character": 29
            }
          ],
          "selectionRange": [
            {
              "line": 402,
              "character": 4
            },
            {
              "line": 402,
              "character": 14
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "LLMClientCompletionStringRequest",
      "detail": "",
      "kind": 22,
      "range": [
        {
          "line": 405,
          "character": 0
        },
        {
          "line": 413,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 406,
          "character": 11
        },
        {
          "line": 406,
          "character": 43
        }
      ],
      "children": [
        {
          "name": "model",
          "detail": "LLMType",
          "kind": 7,
          "range": [
            {
              "line": 407,
              "character": 4
            },
            {
              "line": 407,
              "character": 18
            }
          ],
          "selectionRange": [
            {
              "line": 407,
              "character": 4
            },
            {
              "line": 407,
              "character": 9
            }
          ],
          "children": []
        },
        {
          "name": "prompt",
          "detail": "String",
          "kind": 7,
          "range": [
            {
              "line": 408,
              "character": 4
            },
            {
              "line": 408,
              "character": 18
            }
          ],
          "selectionRange": [
            {
              "line": 408,
              "character": 4
            },
            {
              "line": 408,
              "character": 10
            }
          ],
          "children": []
        },
        {
          "name": "temperature",
          "detail": "f32",
          "kind": 7,
          "range": [
            {
              "line": 409,
              "character": 4
            },
            {
              "line": 409,
              "character": 20
            }
          ],
          "selectionRange": [
            {
              "line": 409,
              "character": 4
            },
            {
              "line": 409,
              "character": 15
            }
          ],
          "children": []
        },
        {
          "name": "frequency_penalty",
          "detail": "Option<f32>",
          "kind": 7,
          "range": [
            {
              "line": 410,
              "character": 4
            },
            {
              "line": 410,
              "character": 34
            }
          ],
          "selectionRange": [
            {
              "line": 410,
              "character": 4
            },
            {
              "line": 410,
              "character": 21
            }
          ],
          "children": []
        },
        {
          "name": "stop_words",
          "detail": "Option<Vec<String>>",
          "kind": 7,
          "range": [
            {
              "line": 411,
              "character": 4
            },
            {
              "line": 411,
              "character": 35
            }
          ],
          "selectionRange": [
            {
              "line": 411,
              "character": 4
            },
            {
              "line": 411,
              "character": 14
            }
          ],
          "children": []
        },
        {
          "name": "max_tokens",
          "detail": "Option<usize>",
          "kind": 7,
          "range": [
            {
              "line": 412,
              "character": 4
            },
            {
              "line": 412,
              "character": 29
            }
          ],
          "selectionRange": [
            {
              "line": 412,
              "character": 4
            },
            {
              "line": 412,
              "character": 14
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl LLMClientCompletionStringRequest",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 415,
          "character": 0
        },
        {
          "line": 465,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 415,
          "character": 5
        },
        {
          "line": 415,
          "character": 37
        }
      ],
      "children": [
        {
          "name": "new",
          "detail": "fn( model: LLMType, prompt: String, temperature: f32, frequency_penalty: Option<f32>, ) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 416,
              "character": 4
            },
            {
              "line": 430,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 416,
              "character": 11
            },
            {
              "line": 416,
              "character": 14
            }
          ],
          "children": []
        },
        {
          "name": "set_stop_words",
          "detail": "fn(mut self, stop_words: Vec<String>) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 432,
              "character": 4
            },
            {
              "line": 435,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 432,
              "character": 11
            },
            {
              "line": 432,
              "character": 25
            }
          ],
          "children": []
        },
        {
          "name": "model",
          "detail": "fn(&self) -> &LLMType",
          "kind": 11,
          "range": [
            {
              "line": 437,
              "character": 4
            },
            {
              "line": 439,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 437,
              "character": 11
            },
            {
              "line": 437,
              "character": 16
            }
          ],
          "children": []
        },
        {
          "name": "temperature",
          "detail": "fn(&self) -> f32",
          "kind": 11,
          "range": [
            {
              "line": 441,
              "character": 4
            },
            {
              "line": 443,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 441,
              "character": 11
            },
            {
              "line": 441,
              "character": 22
            }
          ],
          "children": []
        },
        {
          "name": "frequency_penalty",
          "detail": "fn(&self) -> Option<f32>",
          "kind": 11,
          "range": [
            {
              "line": 445,
              "character": 4
            },
            {
              "line": 447,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 445,
              "character": 11
            },
            {
              "line": 445,
              "character": 28
            }
          ],
          "children": []
        },
        {
          "name": "prompt",
          "detail": "fn(&self) -> &str",
          "kind": 11,
          "range": [
            {
              "line": 449,
              "character": 4
            },
            {
              "line": 451,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 449,
              "character": 11
            },
            {
              "line": 449,
              "character": 17
            }
          ],
          "children": []
        },
        {
          "name": "stop_words",
          "detail": "fn(&self) -> Option<&[String]>",
          "kind": 11,
          "range": [
            {
              "line": 453,
              "character": 4
            },
            {
              "line": 455,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 453,
              "character": 11
            },
            {
              "line": 453,
              "character": 21
            }
          ],
          "children": []
        },
        {
          "name": "set_max_tokens",
          "detail": "fn(mut self, max_tokens: usize) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 457,
              "character": 4
            },
            {
              "line": 460,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 457,
              "character": 11
            },
            {
              "line": 457,
              "character": 25
            }
          ],
          "children": []
        },
        {
          "name": "get_max_tokens",
          "detail": "fn(&self) -> Option<usize>",
          "kind": 11,
          "range": [
            {
              "line": 462,
              "character": 4
            },
            {
              "line": 464,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 462,
              "character": 11
            },
            {
              "line": 462,
              "character": 25
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl LLMClientCompletionRequest",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 467,
          "character": 0
        },
        {
          "line": 569,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 467,
          "character": 5
        },
        {
          "line": 467,
          "character": 31
        }
      ],
      "children": [
        {
          "name": "new",
          "detail": "fn( model: LLMType, messages: Vec<LLMClientMessage>, temperature: f32, frequency_penalty: Option<f32>, ) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 468,
              "character": 4
            },
            {
              "line": 482,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 468,
              "character": 11
            },
            {
              "line": 468,
              "character": 14
            }
          ],
          "children": []
        },
        {
          "name": "set_llm",
          "detail": "fn(mut self, llm: LLMType) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 484,
              "character": 4
            },
            {
              "line": 487,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 484,
              "character": 11
            },
            {
              "line": 484,
              "character": 18
            }
          ],
          "children": []
        },
        {
          "name": "fix_message_structure",
          "detail": "fn(mut self: Self) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 489,
              "character": 4
            },
            {
              "line": 530,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 489,
              "character": 11
            },
            {
              "line": 489,
              "character": 32
            }
          ],
          "children": []
        },
        {
          "name": "from_messages",
          "detail": "fn(messages: Vec<LLMClientMessage>, model: LLMType) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 532,
              "character": 4
            },
            {
              "line": 534,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 532,
              "character": 11
            },
            {
              "line": 532,
              "character": 24
            }
          ],
          "children": []
        },
        {
          "name": "set_temperature",
          "detail": "fn(mut self, temperature: f32) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 536,
              "character": 4
            },
            {
              "line": 539,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 536,
              "character": 11
            },
            {
              "line": 536,
              "character": 26
            }
          ],
          "children": []
        },
        {
          "name": "messages",
          "detail": "fn(&self) -> &[LLMClientMessage]",
          "kind": 11,
          "range": [
            {
              "line": 541,
              "character": 4
            },
            {
              "line": 543,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 541,
              "character": 11
            },
            {
              "line": 541,
              "character": 19
            }
          ],
          "children": []
        },
        {
          "name": "temperature",
          "detail": "fn(&self) -> f32",
          "kind": 11,
          "range": [
            {
              "line": 545,
              "character": 4
            },
            {
              "line": 547,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 545,
              "character": 11
            },
            {
              "line": 545,
              "character": 22
            }
          ],
          "children": []
        },
        {
          "name": "frequency_penalty",
          "detail": "fn(&self) -> Option<f32>",
          "kind": 11,
          "range": [
            {
              "line": 549,
              "character": 4
            },
            {
              "line": 551,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 549,
              "character": 11
            },
            {
              "line": 549,
              "character": 28
            }
          ],
          "children": []
        },
        {
          "name": "model",
          "detail": "fn(&self) -> &LLMType",
          "kind": 11,
          "range": [
            {
              "line": 553,
              "character": 4
            },
            {
              "line": 555,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 553,
              "character": 11
            },
            {
              "line": 553,
              "character": 16
            }
          ],
          "children": []
        },
        {
          "name": "stop_words",
          "detail": "fn(&self) -> Option<&[String]>",
          "kind": 11,
          "range": [
            {
              "line": 557,
              "character": 4
            },
            {
              "line": 559,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 557,
              "character": 11
            },
            {
              "line": 557,
              "character": 21
            }
          ],
          "children": []
        },
        {
          "name": "set_max_tokens",
          "detail": "fn(mut self, max_tokens: usize) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 561,
              "character": 4
            },
            {
              "line": 564,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 561,
              "character": 11
            },
            {
              "line": 561,
              "character": 25
            }
          ],
          "children": []
        },
        {
          "name": "get_max_tokens",
          "detail": "fn(&self) -> Option<usize>",
          "kind": 11,
          "range": [
            {
              "line": 566,
              "character": 4
            },
            {
              "line": 568,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 566,
              "character": 11
            },
            {
              "line": 566,
              "character": 25
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "LLMClientCompletionResponse",
      "detail": "",
      "kind": 22,
      "range": [
        {
          "line": 571,
          "character": 0
        },
        {
          "line": 576,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 572,
          "character": 11
        },
        {
          "line": 572,
          "character": 38
        }
      ],
      "children": [
        {
          "name": "answer_up_until_now",
          "detail": "String",
          "kind": 7,
          "range": [
            {
              "line": 573,
              "character": 4
            },
            {
              "line": 573,
              "character": 31
            }
          ],
          "selectionRange": [
            {
              "line": 573,
              "character": 4
            },
            {
              "line": 573,
              "character": 23
            }
          ],
          "children": []
        },
        {
          "name": "delta",
          "detail": "Option<String>",
          "kind": 7,
          "range": [
            {
              "line": 574,
              "character": 4
            },
            {
              "line": 574,
              "character": 25
            }
          ],
          "selectionRange": [
            {
              "line": 574,
              "character": 4
            },
            {
              "line": 574,
              "character": 9
            }
          ],
          "children": []
        },
        {
          "name": "model",
          "detail": "String",
          "kind": 7,
          "range": [
            {
              "line": 575,
              "character": 4
            },
            {
              "line": 575,
              "character": 17
            }
          ],
          "selectionRange": [
            {
              "line": 575,
              "character": 4
            },
            {
              "line": 575,
              "character": 9
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "impl LLMClientCompletionResponse",
      "detail": "",
      "kind": 18,
      "range": [
        {
          "line": 578,
          "character": 0
        },
        {
          "line": 598,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 578,
          "character": 5
        },
        {
          "line": 578,
          "character": 32
        }
      ],
      "children": [
        {
          "name": "new",
          "detail": "fn(answer_up_until_now: String, delta: Option<String>, model: String) -> Self",
          "kind": 11,
          "range": [
            {
              "line": 579,
              "character": 4
            },
            {
              "line": 585,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 579,
              "character": 11
            },
            {
              "line": 579,
              "character": 14
            }
          ],
          "children": []
        },
        {
          "name": "answer_up_until_now",
          "detail": "fn(&self) -> &str",
          "kind": 11,
          "range": [
            {
              "line": 587,
              "character": 4
            },
            {
              "line": 589,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 587,
              "character": 11
            },
            {
              "line": 587,
              "character": 30
            }
          ],
          "children": []
        },
        {
          "name": "delta",
          "detail": "fn(&self) -> Option<&str>",
          "kind": 11,
          "range": [
            {
              "line": 591,
              "character": 4
            },
            {
              "line": 593,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 591,
              "character": 11
            },
            {
              "line": 591,
              "character": 16
            }
          ],
          "children": []
        },
        {
          "name": "model",
          "detail": "fn(&self) -> &str",
          "kind": 11,
          "range": [
            {
              "line": 595,
              "character": 4
            },
            {
              "line": 597,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 595,
              "character": 11
            },
            {
              "line": 595,
              "character": 16
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "LLMClientError",
      "detail": "",
      "kind": 9,
      "range": [
        {
          "line": 600,
          "character": 0
        },
        {
          "line": 643,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 601,
          "character": 9
        },
        {
          "line": 601,
          "character": 23
        }
      ],
      "children": [
        {
          "name": "FailedToGetResponse",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 602,
              "character": 4
            },
            {
              "line": 603,
              "character": 23
            }
          ],
          "selectionRange": [
            {
              "line": 603,
              "character": 4
            },
            {
              "line": 603,
              "character": 23
            }
          ],
          "children": []
        },
        {
          "name": "ReqwestError",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 605,
              "character": 4
            },
            {
              "line": 606,
              "character": 40
            }
          ],
          "selectionRange": [
            {
              "line": 606,
              "character": 4
            },
            {
              "line": 606,
              "character": 16
            }
          ],
          "children": []
        },
        {
          "name": "SerdeError",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 608,
              "character": 4
            },
            {
              "line": 609,
              "character": 41
            }
          ],
          "selectionRange": [
            {
              "line": 609,
              "character": 4
            },
            {
              "line": 609,
              "character": 14
            }
          ],
          "children": []
        },
        {
          "name": "SendError",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 611,
              "character": 4
            },
            {
              "line": 612,
              "character": 87
            }
          ],
          "selectionRange": [
            {
              "line": 612,
              "character": 4
            },
            {
              "line": 612,
              "character": 13
            }
          ],
          "children": []
        },
        {
          "name": "UnSupportedModel",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 614,
              "character": 4
            },
            {
              "line": 615,
              "character": 20
            }
          ],
          "selectionRange": [
            {
              "line": 615,
              "character": 4
            },
            {
              "line": 615,
              "character": 20
            }
          ],
          "children": []
        },
        {
          "name": "OpenAPIError",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 617,
              "character": 4
            },
            {
              "line": 618,
              "character": 58
            }
          ],
          "selectionRange": [
            {
              "line": 618,
              "character": 4
            },
            {
              "line": 618,
              "character": 16
            }
          ],
          "children": []
        },
        {
          "name": "WrongAPIKeyType",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 620,
              "character": 4
            },
            {
              "line": 621,
              "character": 19
            }
          ],
          "selectionRange": [
            {
              "line": 621,
              "character": 4
            },
            {
              "line": 621,
              "character": 19
            }
          ],
          "children": []
        },
        {
          "name": "OpenAIDoesNotSupportCompletion",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 623,
              "character": 4
            },
            {
              "line": 624,
              "character": 34
            }
          ],
          "selectionRange": [
            {
              "line": 624,
              "character": 4
            },
            {
              "line": 624,
              "character": 34
            }
          ],
          "children": []
        },
        {
          "name": "SqliteSetupError",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 626,
              "character": 4
            },
            {
              "line": 627,
              "character": 20
            }
          ],
          "selectionRange": [
            {
              "line": 627,
              "character": 4
            },
            {
              "line": 627,
              "character": 20
            }
          ],
          "children": []
        },
        {
          "name": "TokioMpscSendError",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 629,
              "character": 4
            },
            {
              "line": 630,
              "character": 22
            }
          ],
          "selectionRange": [
            {
              "line": 630,
              "character": 4
            },
            {
              "line": 630,
              "character": 22
            }
          ],
          "children": []
        },
        {
          "name": "FailedToStoreInDB",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 632,
              "character": 4
            },
            {
              "line": 633,
              "character": 21
            }
          ],
          "selectionRange": [
            {
              "line": 633,
              "character": 4
            },
            {
              "line": 633,
              "character": 21
            }
          ],
          "children": []
        },
        {
          "name": "SqlxError",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 635,
              "character": 4
            },
            {
              "line": 636,
              "character": 34
            }
          ],
          "selectionRange": [
            {
              "line": 636,
              "character": 4
            },
            {
              "line": 636,
              "character": 13
            }
          ],
          "children": []
        },
        {
          "name": "FunctionCallNotPresent",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 638,
              "character": 4
            },
            {
              "line": 639,
              "character": 26
            }
          ],
          "selectionRange": [
            {
              "line": 639,
              "character": 4
            },
            {
              "line": 639,
              "character": 26
            }
          ],
          "children": []
        },
        {
          "name": "GeminiProDoesNotSupportPromptCompletion",
          "detail": "",
          "kind": 21,
          "range": [
            {
              "line": 641,
              "character": 4
            },
            {
              "line": 642,
              "character": 43
            }
          ],
          "selectionRange": [
            {
              "line": 642,
              "character": 4
            },
            {
              "line": 642,
              "character": 43
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "LLMClient",
      "detail": "",
      "kind": 10,
      "range": [
        {
          "line": 645,
          "character": 0
        },
        {
          "line": 668,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 646,
          "character": 10
        },
        {
          "line": 646,
          "character": 19
        }
      ],
      "children": [
        {
          "name": "client",
          "detail": "fn(&self) -> &LLMProvider",
          "kind": 11,
          "range": [
            {
              "line": 647,
              "character": 4
            },
            {
              "line": 647,
              "character": 37
            }
          ],
          "selectionRange": [
            {
              "line": 647,
              "character": 7
            },
            {
              "line": 647,
              "character": 13
            }
          ],
          "children": []
        },
        {
          "name": "stream_completion",
          "detail": "fn( &self, api_key: LLMProviderAPIKeys, request: LLMClientCompletionRequest, sender: UnboundedSender<LLMClientCompletionResponse>, ) -> Result<String, LLMClientError>",
          "kind": 11,
          "range": [
            {
              "line": 649,
              "character": 4
            },
            {
              "line": 654,
              "character": 40
            }
          ],
          "selectionRange": [
            {
              "line": 649,
              "character": 13
            },
            {
              "line": 649,
              "character": 30
            }
          ],
          "children": []
        },
        {
          "name": "completion",
          "detail": "fn( &self, api_key: LLMProviderAPIKeys, request: LLMClientCompletionRequest, ) -> Result<String, LLMClientError>",
          "kind": 11,
          "range": [
            {
              "line": 656,
              "character": 4
            },
            {
              "line": 660,
              "character": 40
            }
          ],
          "selectionRange": [
            {
              "line": 656,
              "character": 13
            },
            {
              "line": 656,
              "character": 23
            }
          ],
          "children": []
        },
        {
          "name": "stream_prompt_completion",
          "detail": "fn( &self, api_key: LLMProviderAPIKeys, request: LLMClientCompletionStringRequest, sender: UnboundedSender<LLMClientCompletionResponse>, ) -> Result<String, LLMClientError>",
          "kind": 11,
          "range": [
            {
              "line": 662,
              "character": 4
            },
            {
              "line": 667,
              "character": 40
            }
          ],
          "selectionRange": [
            {
              "line": 662,
              "character": 13
            },
            {
              "line": 662,
              "character": 37
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "tests",
      "detail": "",
      "kind": 1,
      "range": [
        {
          "line": 670,
          "character": 0
        },
        {
          "line": 680,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 671,
          "character": 4
        },
        {
          "line": 671,
          "character": 9
        }
      ],
      "children": [
        {
          "name": "test_llm_type_from_string",
          "detail": "fn()",
          "kind": 11,
          "range": [
            {
              "line": 674,
              "character": 4
            },
            {
              "line": 679,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 675,
              "character": 7
            },
            {
              "line": 675,
              "character": 32
            }
          ],
          "children": []
        }
      ]
    },
    {
      "name": "something",
      "detail": "",
      "kind": 1,
      "range": [
        {
          "line": 682,
          "character": 0
        },
        {
          "line": 690,
          "character": 1
        }
      ],
      "selectionRange": [
        {
          "line": 682,
          "character": 4
        },
        {
          "line": 682,
          "character": 13
        }
      ],
      "children": [
        {
          "name": "somethingelse",
          "detail": "",
          "kind": 1,
          "range": [
            {
              "line": 683,
              "character": 4
            },
            {
              "line": 689,
              "character": 5
            }
          ],
          "selectionRange": [
            {
              "line": 683,
              "character": 8
            },
            {
              "line": 683,
              "character": 21
            }
          ],
          "children": [
            {
              "name": "internalsomething",
              "detail": "",
              "kind": 1,
              "range": [
                {
                  "line": 684,
                  "character": 8
                },
                {
                  "line": 688,
                  "character": 9
                }
              ],
              "selectionRange": [
                {
                  "line": 684,
                  "character": 12
                },
                {
                  "line": 684,
                  "character": 29
                }
              ],
              "children": [
                {
                  "name": "something",
                  "detail": "fn()",
                  "kind": 11,
                  "range": [
                    {
                      "line": 685,
                      "character": 12
                    },
                    {
                      "line": 687,
                      "character": 13
                    }
                  ],
                  "selectionRange": [
                    {
                      "line": 685,
                      "character": 15
                    },
                    {
                      "line": 685,
                      "character": 24
                    }
                  ],
                  "children": []
                }
              ]
            }
          ]
        }
      ]
    }
  ]
}"#;
    }
}
