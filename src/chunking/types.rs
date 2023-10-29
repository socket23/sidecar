use super::text_document::Range;

/// Some common types which can be reused across calls

#[derive(Debug, Clone)]
pub enum FunctionNodeType {
    // The identifier for the function
    Identifier,
    // The body of the function without the identifier
    Body,
    // The full function with its name and the body
    Function,
}

impl FunctionNodeType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "identifier" => Some(Self::Identifier),
            "body" => Some(Self::Body),
            "function" => Some(Self::Function),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionInformation {
    range: Range,
    r#type: FunctionNodeType,
}

impl FunctionInformation {
    pub fn new(range: Range, r#type: FunctionNodeType) -> Self {
        Self { range, r#type }
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn r#type(&self) -> &FunctionNodeType {
        &self.r#type
    }
}
