use super::text_document::Range;

/// Some common types which can be reused across calls

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionNodeInformation {
    name: String,
    parameters: String,
    body: String,
    return_type: String,
}

impl FunctionNodeInformation {
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_parameters(&mut self, parameters: String) {
        self.parameters = parameters;
    }

    pub fn set_body(&mut self, body: String) {
        self.body = body;
    }

    pub fn set_return_type(&mut self, return_type: String) {
        self.return_type = return_type;
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_parameters(&self) -> &str {
        &self.parameters
    }

    pub fn get_return_type(&self) -> &str {
        &self.return_type
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionNodeType {
    // The identifier for the function
    Identifier,
    // The body of the function without the identifier
    Body,
    // The full function with its name and the body
    Function,
    // The parameters of the function
    Parameters,
    // The return type of the function
    ReturnType,
}

impl FunctionNodeType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "identifier" => Some(Self::Identifier),
            "body" => Some(Self::Body),
            "function" => Some(Self::Function),
            "parameters" => Some(Self::Parameters),
            "return_type" => Some(Self::ReturnType),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionInformation {
    range: Range,
    r#type: FunctionNodeType,
    node_information: Option<FunctionNodeInformation>,
}

impl FunctionInformation {
    pub fn new(range: Range, r#type: FunctionNodeType) -> Self {
        Self {
            range,
            r#type,
            node_information: None,
        }
    }

    pub fn get_node_information(&self) -> Option<&FunctionNodeInformation> {
        self.node_information.as_ref()
    }

    pub fn set_node_information(&mut self, node_information: FunctionNodeInformation) {
        self.node_information = Some(node_information);
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn r#type(&self) -> &FunctionNodeType {
        &self.r#type
    }

    pub fn content(&self, file_content: &str) -> String {
        file_content[self.range().start_byte()..self.range().end_byte()].to_owned()
    }

    pub fn find_function_in_byte_offset<'a>(
        function_blocks: &'a [&'a Self],
        byte_offset: usize,
    ) -> Option<&'a Self> {
        let mut possible_function_block = None;
        for function_block in function_blocks.into_iter() {
            // if the end byte for this block is greater than the current byte
            // position and the start byte is greater than the current bytes
            // position as well, we have our function block
            if !(function_block.range().end_byte() < byte_offset) {
                if function_block.range().start_byte() > byte_offset {
                    break;
                }
                possible_function_block = Some(function_block);
            }
        }
        possible_function_block.copied()
    }

    pub fn get_expanded_selection_range(
        function_bodies: &[&FunctionInformation],
        selection_range: &Range,
    ) -> Range {
        let mut start_position = selection_range.start_position();
        let mut end_position = selection_range.end_position();
        let selection_start_fn_body =
            Self::find_function_in_byte_offset(function_bodies, selection_range.start_byte());
        let selection_end_fn_body =
            Self::find_function_in_byte_offset(function_bodies, selection_range.end_byte());

        // What we are trying to do here is expand our selection to cover the whole
        // function if we have to
        if let Some(selection_start_function) = selection_start_fn_body {
            // check if we can expand the range a bit here
            if start_position.to_byte_offset() > selection_start_function.range().start_byte() {
                start_position = selection_start_function.range().start_position();
            }
            // check if the function block ends after our current selection
            if selection_start_function.range().end_byte() > end_position.to_byte_offset() {
                end_position = selection_start_function.range().end_position();
            }
        }
        if let Some(selection_end_function) = selection_end_fn_body {
            // check if we can expand the start position byte here a bit
            if selection_end_function.range().start_byte() < start_position.to_byte_offset() {
                start_position = selection_end_function.range().start_position();
            }
            if selection_end_function.range().end_byte() > end_position.to_byte_offset() {
                end_position = selection_end_function.range().end_position();
            }
        }
        dbg!(&start_position, &end_position);
        Range::new(start_position, end_position)
    }

    pub fn fold_function_blocks(mut function_blocks: Vec<Self>) -> Vec<Self> {
        // First we sort the function blocks(which are bodies) based on the start
        // index or the end index
        function_blocks.sort_by(|a, b| {
            a.range()
                .start_byte()
                .cmp(&b.range().start_byte())
                .then_with(|| b.range().end_byte().cmp(&a.range().end_byte()))
        });

        // Now that these are sorted we only keep the ones which are not overlapping
        // or fully contained in the other one
        let mut filtered_function_blocks = Vec::new();
        let mut index = 0;

        while index < function_blocks.len() {
            filtered_function_blocks.push(function_blocks[index].clone());
            let mut iterate_index = index + 1;
            while iterate_index < function_blocks.len()
                && function_blocks[index]
                    .range()
                    .is_contained(&function_blocks[iterate_index].range())
            {
                iterate_index += 1;
            }
            index = iterate_index;
        }

        filtered_function_blocks
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassNodeType {
    Identifier,
    ClassDeclaration,
}

impl ClassNodeType {
    pub fn from_str(s: &str) -> Option<ClassNodeType> {
        match s {
            "identifier" => Some(Self::Identifier),
            "class_declaration" => Some(Self::ClassDeclaration),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClassInformation {
    range: Range,
    name: String,
    class_node_type: ClassNodeType,
}

impl ClassInformation {
    pub fn new(range: Range, name: String, class_node_type: ClassNodeType) -> Self {
        Self {
            range,
            name,
            class_node_type,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn get_class_type(&self) -> &ClassNodeType {
        &self.class_node_type
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn content(&self, content: &str) -> String {
        content[self.range().start_byte()..self.range().end_byte()].to_string()
    }

    pub fn fold_class_information(mut classes: Vec<Self>) -> Vec<Self> {
        // First we sort the function blocks(which are bodies) based on the start
        // index or the end index
        classes.sort_by(|a, b| {
            a.range()
                .start_byte()
                .cmp(&b.range().start_byte())
                .then_with(|| b.range().end_byte().cmp(&a.range().end_byte()))
        });

        // Now that these are sorted we only keep the ones which are not overlapping
        // or fully contained in the other one
        let mut filtered_classes = Vec::new();
        let mut index = 0;

        while index < classes.len() {
            filtered_classes.push(classes[index].clone());
            let mut iterate_index = index + 1;
            while iterate_index < classes.len()
                && classes[index]
                    .range()
                    .is_contained(&classes[iterate_index].range())
            {
                iterate_index += 1;
            }
            index = iterate_index;
        }

        filtered_classes
    }
}

#[derive(Debug, Clone)]
pub struct ClassWithFunctions {
    pub class_information: Option<ClassInformation>,
    pub function_information: Vec<FunctionInformation>,
}

impl ClassWithFunctions {
    pub fn class_functions(
        class_information: ClassInformation,
        function_information: Vec<FunctionInformation>,
    ) -> Self {
        Self {
            class_information: Some(class_information),
            function_information,
        }
    }

    pub fn functions(function_information: Vec<FunctionInformation>) -> Self {
        Self {
            class_information: None,
            function_information,
        }
    }
}
