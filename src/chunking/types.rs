/// Some common types which can be reused across calls

#[derive(Debug)]
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

#[derive(Debug)]
pub struct FunctionInformation<'a> {
    node: tree_sitter::Node<'a>,
    r#type: FunctionNodeType,
}

impl<'a> FunctionInformation<'a> {
    pub fn new(node: tree_sitter::Node<'a>, r#type: FunctionNodeType) -> Self {
        Self { node, r#type }
    }

    pub fn node(&self) -> &tree_sitter::Node<'a> {
        &self.node
    }

    pub fn r#type(&self) -> &FunctionNodeType {
        &self.r#type
    }
}
