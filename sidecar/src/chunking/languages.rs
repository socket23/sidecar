use std::{collections::HashSet, path::Path};

use crate::chunking::types::FunctionNodeInformation;

use super::{
    javascript::javascript_language_config,
    python::python_language_config,
    rust::rust_language_config,
    text_document::{Position, Range},
    types::{
        ClassInformation, ClassNodeType, ClassWithFunctions, FunctionInformation, FunctionNodeType,
        TypeInformation, TypeNodeType,
    },
    typescript::typescript_language_config,
};

fn naive_chunker(buffer: &str, line_count: usize, overlap: usize) -> Vec<Span> {
    let mut chunks: Vec<Span> = vec![];
    let current_chunk = buffer
        .lines()
        .into_iter()
        .map(|line| line.to_owned())
        .collect::<Vec<_>>();
    let chunk_length = current_chunk.len();
    let mut start = 0;
    while start < chunk_length {
        let end = (start + line_count).min(chunk_length);
        let chunk = current_chunk[start..end].to_owned();
        let span = Span::new(start, end, None, Some(chunk.join("\n")));
        chunks.push(span);
        start += line_count - overlap;
    }
    chunks
}

fn get_string_from_bytes(source_code: &Vec<u8>, start_byte: usize, end_byte: usize) -> String {
    String::from_utf8(source_code[start_byte..end_byte].to_vec()).unwrap_or_default()
}

/// We are going to use tree-sitter to parse the code and get the chunks for the
/// code. we are going to use the algo sweep uses for tree-sitter
///
#[derive(Debug, Clone)]
pub struct TSLanguageConfig {
    /// A list of language names that can be processed by these scope queries
    /// e.g.: ["Typescript", "TSX"], ["Rust"]
    pub language_ids: &'static [&'static str],

    /// Extensions that can help classify the file: rs, js, tx, py, etc
    pub file_extensions: &'static [&'static str],

    /// tree-sitter grammar for this language
    pub grammar: fn() -> tree_sitter::Language,

    /// Namespaces defined by this language,
    /// E.g.: type namespace, variable namespace, function namespace
    pub namespaces: Vec<Vec<String>>,

    /// The documentation query which will be used by this language
    pub documentation_query: Vec<String>,

    /// The queries to get the function body for the language
    pub function_query: Vec<String>,

    /// The different constructs for the language and their tree-sitter node types
    pub construct_types: Vec<String>,

    /// The different expression statements which are present in the language
    pub expression_statements: Vec<String>,

    /// The queries we use to get the class definitions
    pub class_query: Vec<String>,

    pub r#type_query: Vec<String>,

    /// The namespaces of the symbols which can be applied to a code symbols
    /// in case of typescript it can be `export` keyword
    pub namespace_types: Vec<String>,

    /// Hoverable queries are used to get identifier which we can hover over
    /// or written another way these are the important parts of the document
    /// which a user can move their marker over and get back data
    pub hoverable_query: String,

    /// What are the different scopes which are present in the language we can
    /// infer that using the scope query to get the local definitions and the
    /// scopes which should be hoisted upwards
    pub scope_query: String,

    /// The comment prefix for the language, typescript is like // and rust
    /// is like //, python is like #
    pub comment_prefix: String,

    /// This is used to keep track of the end of line situations in many lanaguages
    /// if they exist
    pub end_of_line: Option<String>,
}

impl TSLanguageConfig {
    pub fn get_language(&self) -> Option<String> {
        self.language_ids.first().map(|s| s.to_string())
    }

    pub fn is_valid_code(&self, code: &str) -> bool {
        let grammar = self.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let tree_maybe = parser.parse(code, None);
        tree_maybe
            .map(|tree| !tree.root_node().has_error())
            .unwrap_or_default()
    }

    pub fn generate_file_symbols(&self, source_code: &[u8]) -> Vec<ClassWithFunctions> {
        let function_ranges = self.capture_function_data(source_code);
        let class_ranges = self.capture_class_data(source_code);
        let mut classes_with_functions = Vec::new();
        let mut standalone_functions = Vec::new();

        // This is where we maintain the list of functions which we have already
        // added to a class
        let mut added_functions = vec![false; function_ranges.len()];

        for class in class_ranges {
            let mut functions = Vec::new();

            for (i, function) in function_ranges.iter().enumerate() {
                if function.range().start_byte() >= class.range().start_byte()
                    && function.range().end_byte() <= class.range().end_byte()
                    && function.get_node_information().is_some()
                {
                    functions.push(function.clone());
                    added_functions[i] = true; // Mark function as added
                }
            }

            classes_with_functions.push(ClassWithFunctions::class_functions(class, functions));
        }

        // Add standalone functions, those which are not within any class range
        for (i, function) in function_ranges.iter().enumerate() {
            if !added_functions[i] && function.get_node_information().is_some() {
                standalone_functions.push(function.clone());
            }
        }

        classes_with_functions.push(ClassWithFunctions::functions(standalone_functions));
        classes_with_functions
    }

    // The file outline looks like this:
    // function something(arguments): return_value_something
    // Class something_else
    //    function inner_function(arguments_here): return_value_function
    //    function something_else(arguments_here): return_value_something_here
    // ...
    // We will generate a proper outline later on, but for now work with this
    // TODO(skcd): This can be greatly improved here
    pub fn generate_file_outline_str(&self, source_code: &[u8]) -> String {
        let function_ranges = self.capture_function_data(source_code);
        let class_ranges = self.capture_class_data(source_code);
        let language = self
            .get_language()
            .expect("to have some language")
            .to_lowercase();
        let mut outline = format!("```{language}\n");

        // This is where we maintain the list of functions which we have already
        // printed out
        let mut printed_functions = vec![false; function_ranges.len()];

        for class in class_ranges {
            let class_name = class.get_name();
            outline = outline + "\n" + &format!("Class {class_name}") + "\n";
            // Find and print functions within the class range
            for (i, function) in function_ranges.iter().enumerate() {
                if function.range().start_byte() >= class.range().start_byte()
                    && function.range().end_byte() <= class.range().end_byte()
                    && function.get_node_information().is_some()
                {
                    let node_information = function
                        .get_node_information()
                        .expect("AND check above to hold");
                    outline = outline
                        + "\n"
                        + &format!(
                            "    function {} {} {}",
                            node_information.get_name(),
                            node_information.get_parameters(),
                            node_information.get_return_type()
                        );
                    printed_functions[i] = true; // Mark function as printed
                }
            }
        }

        // Print standalone functions, those which are not within any class range
        for (i, function) in function_ranges.iter().enumerate() {
            if !printed_functions[i] && function.get_node_information().is_some() {
                let node_information = function
                    .get_node_information()
                    .expect("AND check above to hold");
                // Check if the function has not been printed yet
                outline = outline
                    + "\n"
                    + &format!(
                        "function {} {} {}",
                        node_information.get_name(),
                        node_information.get_parameters(),
                        node_information.get_return_type()
                    )
                    + "\n";
            }
        }

        outline = outline + "\n" + "```";
        outline
    }

    pub fn capture_documentation_queries(&self, source_code: &[u8]) -> Vec<(Range, String)> {
        // Now we try to grab the documentation strings so we can add them to the functions as well
        let mut parser = tree_sitter::Parser::new();
        let grammar = self.grammar;
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(source_code, None).unwrap();
        let node = parsed_data.root_node();
        let mut range_set = HashSet::new();
        let documentation_queries = self.documentation_query.to_vec();
        let source_code_vec = source_code.to_vec();
        // We want to capture here the range of the comment line and the comment content
        // we can then concat this with the function itself and expand te range of the function
        // node so it covers this comment as well
        let mut documentation_string_information: Vec<(Range, String)> = vec![];
        documentation_queries
            .into_iter()
            .for_each(|documentation_query| {
                let query = tree_sitter::Query::new(grammar(), &documentation_query)
                    .expect("documentation queries are well formed");
                let mut cursor = tree_sitter::QueryCursor::new();
                cursor
                    .captures(&query, node, source_code)
                    .into_iter()
                    .for_each(|capture| {
                        capture.0.captures.into_iter().for_each(|capture| {
                            if !range_set.contains(&Range::for_tree_node(&capture.node)) {
                                let documentation_string = get_string_from_bytes(
                                    &source_code_vec,
                                    capture.node.start_byte(),
                                    capture.node.end_byte(),
                                );
                                documentation_string_information.push((
                                    Range::for_tree_node(&capture.node),
                                    documentation_string,
                                ));
                                range_set.insert(Range::for_tree_node(&capture.node));
                            }
                        })
                    });
            });
        documentation_string_information
    }

    pub fn capture_type_data(&self, source_code: &[u8]) -> Vec<TypeInformation> {
        let type_queries = self.type_query.to_vec();

        let grammar = self.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(source_code, None).unwrap();
        let node = parsed_data.root_node();

        let mut type_nodes = vec![];
        let mut range_set = HashSet::new();
        type_queries.into_iter().for_each(|type_query| {
            let query = tree_sitter::Query::new(grammar(), &type_query)
                .expect("type queries are well formed");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor
                .captures(&query, node, source_code)
                .into_iter()
                .for_each(|capture| {
                    capture.0.captures.into_iter().for_each(|capture| {
                        let capture_name = query
                            .capture_names()
                            .to_vec()
                            .remove(capture.index.try_into().unwrap());
                        let capture_type = TypeNodeType::from_str(&capture_name);
                        if !range_set.contains(&Range::for_tree_node(&capture.node)) {
                            if let Some(capture_type) = capture_type {
                                if capture_type == TypeNodeType::TypeDeclaration {
                                    // if we have the type declaration here, we want to check if
                                    // we should go to the parent of this node and check if its
                                    // an export stament here, since if that's the case
                                    // we want to handle that too
                                    let parent_node = capture.node.parent();
                                    if let Some(parent_node) = parent_node {
                                        if self
                                            .namespace_types
                                            .contains(&parent_node.kind().to_owned())
                                        {
                                            type_nodes.push(TypeInformation::new(
                                                Range::for_tree_node(&parent_node),
                                                "not_set_parent_node".to_owned(),
                                                capture_type,
                                            ));
                                            // to the range set we add the range of the current capture node
                                            range_set.insert(Range::for_tree_node(&capture.node));
                                            return;
                                        }
                                    }
                                }
                                type_nodes.push(TypeInformation::new(
                                    Range::for_tree_node(&capture.node),
                                    "not_set".to_owned(),
                                    capture_type,
                                ));
                                range_set.insert(Range::for_tree_node(&capture.node));
                            }
                        }
                    })
                })
        });

        // Now we iterate again and try to get the name of the types as well
        // and generate the final representation
        // the nodes are ordered in this way:
        // type_node
        // - identifier
        let mut index = 0;
        let mut compressed_types = vec![];
        while index < type_nodes.len() {
            let start_index = index;
            if type_nodes[start_index].get_type_type() != &TypeNodeType::TypeDeclaration {
                index += 1;
                continue;
            }
            compressed_types.push(type_nodes[start_index].clone());
            let mut end_index = start_index + 1;
            let mut type_identifier = None;
            while end_index < type_nodes.len()
                && type_nodes[end_index].get_type_type() != &TypeNodeType::TypeDeclaration
            {
                match type_nodes[end_index].get_type_type() {
                    TypeNodeType::Identifier => {
                        type_identifier = Some(get_string_from_bytes(
                            &source_code.to_vec(),
                            type_nodes[end_index].range().start_byte(),
                            type_nodes[end_index].range().end_byte(),
                        ));
                    }
                    _ => {}
                }
                end_index += 1;
            }

            match (compressed_types.last_mut(), type_identifier) {
                (Some(type_information), Some(type_name)) => {
                    type_information.set_name(type_name);
                }
                _ => {}
            }
            index = end_index;
        }
        let documentation_strings = self.capture_documentation_queries(source_code);
        TypeInformation::add_documentation_to_types(compressed_types, documentation_strings)
    }

    pub fn capture_class_data(&self, source_code: &[u8]) -> Vec<ClassInformation> {
        let class_queries = self.class_query.to_vec();

        let grammar = self.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(source_code, None).unwrap();
        let node = parsed_data.root_node();

        let mut class_nodes = vec![];
        let class_code_vec = source_code.to_vec();
        let mut range_set = HashSet::new();
        class_queries.into_iter().for_each(|class_query| {
            let query = tree_sitter::Query::new(grammar(), &class_query)
                .expect("class queries are well formed");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor
                .captures(&query, node, source_code)
                .into_iter()
                .for_each(|capture| {
                    capture.0.captures.into_iter().for_each(|capture| {
                        let capture_name = query
                            .capture_names()
                            .to_vec()
                            .remove(capture.index.try_into().unwrap());
                        let capture_type = ClassNodeType::from_str(&capture_name);
                        if !range_set.contains(&Range::for_tree_node(&capture.node)) {
                            if let Some(capture_type) = capture_type {
                                // if we have the type declaration here, we want to check if
                                // we should go to the parent of this node and check if its
                                // an export stament here, since if that's the case
                                // we want to handle that too
                                let parent_node = capture.node.parent();
                                if let Some(parent_node) = parent_node {
                                    if self
                                        .namespace_types
                                        .contains(&parent_node.kind().to_owned())
                                    {
                                        class_nodes.push(ClassInformation::new(
                                            Range::for_tree_node(&capture.node),
                                            "not_set_parent".to_owned(),
                                            capture_type,
                                        ));
                                        // to the range set we add the range of the current capture node
                                        range_set.insert(Range::for_tree_node(&capture.node));
                                        return;
                                    }
                                };
                                class_nodes.push(ClassInformation::new(
                                    Range::for_tree_node(&capture.node),
                                    "not_set".to_owned(),
                                    capture_type,
                                ));
                                range_set.insert(Range::for_tree_node(&capture.node));
                            }
                        }
                    })
                })
        });

        // Now we iterate again and try to get the name of the classes as well
        // and generate the final representation
        // the nodes are ordered in this way:
        // class
        // - identifier
        let mut index = 0;
        let mut compressed_classes = vec![];
        while index < class_nodes.len() {
            let start_index = index;
            if class_nodes[start_index].get_class_type() != &ClassNodeType::ClassDeclaration {
                index += 1;
                continue;
            }
            compressed_classes.push(class_nodes[start_index].clone());
            let mut end_index = start_index + 1;
            let mut class_identifier = None;
            while end_index < class_nodes.len()
                && class_nodes[end_index].get_class_type() != &ClassNodeType::ClassDeclaration
            {
                match class_nodes[end_index].get_class_type() {
                    ClassNodeType::Identifier => {
                        class_identifier = Some(get_string_from_bytes(
                            &class_code_vec,
                            class_nodes[end_index].range().start_byte(),
                            class_nodes[end_index].range().end_byte(),
                        ));
                    }
                    _ => {}
                }
                end_index += 1;
            }

            match (compressed_classes.last_mut(), class_identifier) {
                (Some(class_information), Some(class_name)) => {
                    class_information.set_name(class_name);
                }
                _ => {}
            }
            index = end_index;
        }
        let documentation_string_information: Vec<(Range, String)> =
            self.capture_documentation_queries(source_code);
        ClassInformation::add_documentation_to_classes(
            compressed_classes,
            documentation_string_information,
        )
    }

    pub fn capture_function_data(&self, source_code: &[u8]) -> Vec<FunctionInformation> {
        let function_queries = self.function_query.to_vec();
        // We want to capture the function information here and then do a folding on top of
        // it, we just want to keep top level functions over here
        // Now we need to run the tree sitter query on this and get back the
        // answer
        let grammar = self.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(source_code, None).unwrap();
        let node = parsed_data.root_node();
        let mut function_nodes = vec![];
        let source_code_vec = source_code.to_vec();
        let mut range_set = HashSet::new();
        function_queries.into_iter().for_each(|function_query| {
            let query = tree_sitter::Query::new(grammar(), &function_query)
                .expect("function queries are well formed");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor
                .captures(&query, node, source_code)
                .into_iter()
                .for_each(|capture| {
                    capture.0.captures.into_iter().for_each(|capture| {
                        let capture_name = query
                            .capture_names()
                            .to_vec()
                            .remove(capture.index.try_into().unwrap());
                        let capture_type = FunctionNodeType::from_str(&capture_name);
                        if !range_set.contains(&Range::for_tree_node(&capture.node)) {
                            if let Some(capture_type) = capture_type {
                                if capture_type == FunctionNodeType::Function {
                                    // if we have the type declaration here, we want to check if
                                    // we should go to the parent of this node and check if its
                                    // an export stament here, since if that's the case
                                    // we want to handle that too
                                    let parent_node = capture.node.parent();
                                    if let Some(parent_node) = parent_node {
                                        if self
                                            .namespace_types
                                            .contains(&parent_node.kind().to_owned())
                                        {
                                            function_nodes.push(FunctionInformation::new(
                                                Range::for_tree_node(&parent_node),
                                                capture_type,
                                            ));
                                            // to the range set we add the range of the current capture node
                                            range_set.insert(Range::for_tree_node(&capture.node));
                                            return;
                                        }
                                    }
                                }
                                function_nodes.push(FunctionInformation::new(
                                    Range::for_tree_node(&capture.node),
                                    capture_type,
                                ));
                                range_set.insert(Range::for_tree_node(&capture.node));
                            }
                        }
                    })
                });
        });

        // Now we know from the query, that we have to do the following:
        // function
        // - identifier
        // - body
        // - parameters
        // - return
        let mut index = 0;
        let mut compressed_functions = vec![];
        while index < function_nodes.len() {
            let start_index = index;
            if function_nodes[start_index].r#type() != &FunctionNodeType::Function {
                index += 1;
                continue;
            }
            compressed_functions.push(function_nodes[start_index].clone());
            let mut end_index = start_index + 1;
            let mut function_node_information = FunctionNodeInformation::default();
            while end_index < function_nodes.len()
                && function_nodes[end_index].r#type() != &FunctionNodeType::Function
            {
                match function_nodes[end_index].r#type() {
                    &FunctionNodeType::Identifier => {
                        function_node_information.set_name(get_string_from_bytes(
                            &source_code_vec,
                            function_nodes[end_index].range().start_byte(),
                            function_nodes[end_index].range().end_byte(),
                        ));
                    }
                    &FunctionNodeType::Body => {
                        function_node_information.set_body(get_string_from_bytes(
                            &source_code_vec,
                            function_nodes[end_index].range().start_byte(),
                            function_nodes[end_index].range().end_byte(),
                        ));
                    }
                    &FunctionNodeType::Parameters => {
                        function_node_information.set_parameters(get_string_from_bytes(
                            &source_code_vec,
                            function_nodes[end_index].range().start_byte(),
                            function_nodes[end_index].range().end_byte(),
                        ));
                    }
                    &FunctionNodeType::ReturnType => {
                        function_node_information.set_return_type(get_string_from_bytes(
                            &source_code_vec,
                            function_nodes[end_index].range().start_byte(),
                            function_nodes[end_index].range().end_byte(),
                        ));
                    }
                    _ => {}
                }
                end_index += 1;
            }

            match compressed_functions.last_mut() {
                Some(function_information) => {
                    function_information.set_node_information(function_node_information);
                }
                None => {}
            }
            index = end_index;
        }

        let mut documentation_string_information: Vec<(Range, String)> =
            self.capture_documentation_queries(source_code);
        // Now we want to append the documentation string to the functions
        FunctionInformation::add_documentation_to_functions(
            FunctionInformation::fold_function_blocks(compressed_functions),
            documentation_string_information,
        )
    }

    pub fn function_information_nodes(&self, source_code: &[u8]) -> Vec<FunctionInformation> {
        let function_queries = self.function_query.to_vec();

        // Now we need to run the tree sitter query on this and get back the
        // answer
        let grammar = self.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(source_code, None).unwrap();
        let node = parsed_data.root_node();
        let mut function_nodes = vec![];
        let mut unique_ranges: HashSet<tree_sitter::Range> = Default::default();
        function_queries.into_iter().for_each(|function_query| {
            let query = tree_sitter::Query::new(grammar(), &function_query)
                .expect("function queries are well formed");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor
                .captures(&query, node, source_code)
                .into_iter()
                .for_each(|capture| {
                    capture.0.captures.into_iter().for_each(|capture| {
                        let capture_name = query
                            .capture_names()
                            .to_vec()
                            .remove(capture.index.try_into().unwrap());
                        let capture_type = FunctionNodeType::from_str(&capture_name);
                        if let Some(capture_type) = capture_type {
                            function_nodes.push(FunctionInformation::new(
                                Range::for_tree_node(&capture.node),
                                capture_type,
                            ));
                        }
                    })
                });
        });
        function_nodes
            .into_iter()
            .filter_map(|function_node| {
                let range = function_node.range();
                if unique_ranges.contains(&range.to_tree_sitter_range()) {
                    return None;
                }
                unique_ranges.insert(range.to_tree_sitter_range());
                Some(function_node.clone())
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct TSLanguageParsing {
    configs: Vec<TSLanguageConfig>,
}

impl TSLanguageParsing {
    pub fn init() -> Self {
        Self {
            configs: vec![
                javascript_language_config(),
                typescript_language_config(),
                rust_language_config(),
                python_language_config(),
            ],
        }
    }

    pub fn for_lang(&self, language: &str) -> Option<&TSLanguageConfig> {
        self.configs
            .iter()
            .find(|config| config.language_ids.contains(&language))
    }

    /// We will use this to chunk the file to pieces which can be used for
    /// searching
    pub fn chunk_file(
        &self,
        file_path: &str,
        buffer: &str,
        file_extension: Option<&str>,
        file_language_id: Option<&str>,
    ) -> Vec<Span> {
        if file_extension.is_none() && file_language_id.is_none() {
            // We use naive chunker here which just splits on the number
            // of lines
            return naive_chunker(buffer, 30, 15);
        }
        let mut language_config_maybe = None;
        if let Some(language_id) = file_language_id {
            language_config_maybe = self.for_lang(language_id);
        }
        if let Some(file_extension) = file_extension {
            language_config_maybe = self
                .configs
                .iter()
                .find(|config| config.file_extensions.contains(&file_extension));
        }
        if let Some(language_config) = language_config_maybe {
            // We use tree-sitter to parse the file and get the chunks
            // for the file
            let language = language_config.grammar;
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(language()).unwrap();
            let tree = parser.parse(buffer.as_bytes(), None).unwrap();
            // we allow for 1500 characters and 100 character coalesce
            let chunks = chunk_tree(&tree, language_config, 1500, 100, &buffer);
            chunks
        } else {
            // use naive chunker here which just splits the file into parts
            return naive_chunker(buffer, 30, 15);
        }
    }

    pub fn parse_documentation(&self, code: &str, language: &str) -> Vec<String> {
        let language_config_maybe = self
            .configs
            .iter()
            .find(|config| config.language_ids.contains(&language));
        if let None = language_config_maybe {
            return Default::default();
        }
        let language_config = language_config_maybe.expect("if let None check above to hold");
        let grammar = language_config.grammar;
        let documentation_queries = language_config.documentation_query.to_vec();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(code, None).unwrap();
        let node = parsed_data.root_node();
        let mut nodes = vec![];
        documentation_queries
            .into_iter()
            .for_each(|documentation_query| {
                let query = tree_sitter::Query::new(grammar(), &documentation_query)
                    .expect("documentation queries are well formed");
                let mut cursor = tree_sitter::QueryCursor::new();
                cursor
                    .captures(&query, node, code.as_bytes())
                    .into_iter()
                    .for_each(|capture| {
                        capture.0.captures.into_iter().for_each(|capture| {
                            nodes.push(capture.node);
                        })
                    });
            });

        // Now we only want to keep the unique ranges which we have captured
        // from the nodes
        let mut node_ranges: HashSet<tree_sitter::Range> = Default::default();
        let nodes = nodes
            .into_iter()
            .filter(|capture| {
                let range = capture.range();
                if node_ranges.contains(&range) {
                    return false;
                }
                node_ranges.insert(range);
                true
            })
            .collect::<Vec<_>>();

        // Now that we have the nodes, we also want to merge them together,
        // for that we need to first order the nodes
        get_merged_documentation_nodes(nodes, code)
    }

    pub fn function_information_nodes(
        &self,
        source_code: &str,
        language: &str,
    ) -> Vec<FunctionInformation> {
        let language_config = self.for_lang(language);
        if let None = language_config {
            return Default::default();
        }
        let language_config = language_config.expect("if let None check above to hold");
        let function_queries = language_config.function_query.to_vec();

        // Now we need to run the tree sitter query on this and get back the
        // answer
        let grammar = language_config.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(source_code.as_bytes(), None).unwrap();
        let node = parsed_data.root_node();
        let mut function_nodes = vec![];
        let mut unique_ranges: HashSet<tree_sitter::Range> = Default::default();
        function_queries.into_iter().for_each(|function_query| {
            let query = tree_sitter::Query::new(grammar(), &function_query)
                .expect("function queries are well formed");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor
                .captures(&query, node, source_code.as_bytes())
                .into_iter()
                .for_each(|capture| {
                    capture.0.captures.into_iter().for_each(|capture| {
                        let capture_name = query
                            .capture_names()
                            .to_vec()
                            .remove(capture.index.try_into().unwrap());
                        let capture_type = FunctionNodeType::from_str(&capture_name);
                        if let Some(capture_type) = capture_type {
                            function_nodes.push(FunctionInformation::new(
                                Range::for_tree_node(&capture.node),
                                capture_type,
                            ));
                        }
                    })
                });
        });
        function_nodes
            .into_iter()
            .filter_map(|function_node| {
                let range = function_node.range();
                if unique_ranges.contains(&range.to_tree_sitter_range()) {
                    return None;
                }
                unique_ranges.insert(range.to_tree_sitter_range());
                Some(function_node.clone())
            })
            .collect()
    }

    pub fn get_fix_range<'a>(
        &'a self,
        source_code: &'a str,
        language: &'a str,
        range: &'a Range,
        extra_width: usize,
    ) -> Option<Range> {
        let language_config = self.for_lang(language);
        if let None = language_config {
            return None;
        }
        let language_config = language_config.expect("if let None check above to hold");
        let grammar = language_config.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(source_code.as_bytes(), None).unwrap();
        let node = parsed_data.root_node();
        let descendant_node_maybe =
            node.descendant_for_byte_range(range.start_byte(), range.end_byte());
        if let None = descendant_node_maybe {
            return None;
        }
        // we are going to now check if the descendant node is important enough
        // for us to consider and fits in the size range we expect it to
        let descendant_node = descendant_node_maybe.expect("if let None to hold");
        let mut cursor = descendant_node.walk();
        let children: Vec<_> = descendant_node
            .named_children(&mut cursor)
            .into_iter()
            .collect();
        let found_range = iterate_over_nodes_within_range(
            language,
            descendant_node,
            extra_width,
            range,
            true,
            language_config,
        );
        let current_node_range = Range::for_tree_node(&descendant_node);
        if found_range.start_byte() == current_node_range.start_byte()
            && found_range.end_byte() == current_node_range.end_byte()
        {
            // here we try to iterate upwards if we can find a node
            Some(find_node_to_use(language, descendant_node, language_config))
        } else {
            Some(found_range)
        }
    }

    pub fn get_parent_range_for_selection(
        &self,
        source_code: &str,
        language: &str,
        range: &Range,
    ) -> Range {
        let language_config = self.for_lang(language);
        if let None = language_config {
            return range.clone();
        }
        let language_config = language_config.expect("if let None check above to hold");
        let grammar = language_config.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(source_code.as_bytes(), None).unwrap();
        let node = parsed_data.root_node();
        let query = language_config
            .construct_types
            .iter()
            .map(|construct_type| format!("({construct_type}) @scope"))
            .collect::<Vec<_>>()
            .join("\n");
        let query = tree_sitter::Query::new(grammar(), &query).expect("query to be well formed");
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut found_node = None;
        cursor
            .matches(&query, node, source_code.as_bytes())
            .into_iter()
            .for_each(|capture| {
                capture.captures.into_iter().for_each(|capture| {
                    let node = capture.node;
                    let node_range = Range::for_tree_node(&node);
                    if node_range.start_byte() <= range.start_byte()
                        && node_range.end_byte() >= range.end_byte()
                        && found_node.is_none()
                    {
                        found_node = Some(node);
                    }
                })
            });
        found_node
            .map(|node| Range::for_tree_node(&node))
            .unwrap_or(range.clone())
    }

    pub fn detect_lang(&self, path: &str) -> Option<String> {
        // Here we look at the path extension from path and use that for detecting
        // the language
        Path::new(path)
            .extension()
            .map(|extension| extension.to_str())
            .flatten()
            .map(|ext| ext.to_string())
    }
}

fn find_node_to_use(
    language: &str,
    node: tree_sitter::Node<'_>,
    language_config: &TSLanguageConfig,
) -> Range {
    let parent_node = node.parent();
    let current_range = Range::for_tree_node(&node);
    let construct_type = language_config
        .construct_types
        .contains(&node.kind().to_owned());
    if construct_type || parent_node.is_none() {
        return current_range;
    }
    let parent_node = parent_node.expect("check above to work");
    let filtered_ranges = keep_iterating(
        parent_node
            .children(&mut parent_node.walk())
            .into_iter()
            .collect::<Vec<_>>(),
        parent_node,
        language_config,
        false,
    );
    if filtered_ranges.is_none() {
        return current_range;
    }
    let filtered_ranges_with_interest_node = filtered_ranges.expect("if let is_none to work");
    let filtered_ranges = filtered_ranges_with_interest_node.filtered_nodes;
    let index_of_interest = filtered_ranges_with_interest_node.index_of_interest;
    let index_of_interest_i64 = <i64>::try_from(index_of_interest).expect("usize to i64 to work");
    if index_of_interest_i64 - 1 >= 0
        && index_of_interest_i64 <= <i64>::try_from(filtered_ranges.len()).unwrap() - 1
    {
        let before_node = filtered_ranges[index_of_interest - 1];
        let after_node = filtered_ranges[index_of_interest + 1];
        Range::new(
            Position::from_tree_sitter_point(
                &before_node.start_position(),
                before_node.start_byte(),
            ),
            Position::from_tree_sitter_point(&after_node.end_position(), after_node.end_byte()),
        )
    } else {
        find_node_to_use(language, parent_node, language_config)
    }
}

fn iterate_over_nodes_within_range(
    language: &str,
    node: tree_sitter::Node<'_>,
    line_limit: usize,
    range: &Range,
    should_go_inside: bool,
    language_config: &TSLanguageConfig,
) -> Range {
    let children = node
        .children(&mut node.walk())
        .into_iter()
        .collect::<Vec<_>>();
    if node.range().end_point.row - node.range().start_point.row + 1 <= line_limit {
        let found_range = if language_config
            .construct_types
            .contains(&node.kind().to_owned())
        {
            // if we have a matching kind, then we should be probably looking at
            // this node which fits the bill and keep going
            return Range::for_tree_node(&node);
        } else {
            iterate_over_children(
                language,
                children,
                line_limit,
                node,
                language_config,
                should_go_inside,
            )
        };
        let parent_node = node.parent();
        if let None = parent_node {
            found_range
        } else {
            let mut parent = parent_node.expect("if let None to hold");
            // we iterate over the children of the parent
            iterate_over_nodes_within_range(
                language,
                parent,
                line_limit,
                &found_range,
                false,
                language_config,
            )
        }
    } else {
        iterate_over_children(
            language,
            children,
            line_limit,
            node,
            language_config,
            should_go_inside,
        )
    }
}

fn iterate_over_children(
    language: &str,
    children: Vec<tree_sitter::Node<'_>>,
    line_limit: usize,
    some_other_node_to_name: tree_sitter::Node<'_>,
    language_config: &TSLanguageConfig,
    should_go_inside: bool,
) -> Range {
    if children.is_empty() {
        return Range::for_tree_node(&some_other_node_to_name);
    }
    let filtered_ranges_maybe = keep_iterating(
        children,
        some_other_node_to_name,
        language_config,
        should_go_inside,
    );

    if let None = filtered_ranges_maybe {
        return Range::for_tree_node(&some_other_node_to_name);
    }

    let filtered_range = filtered_ranges_maybe.expect("if let None");
    let interested_nodes = filtered_range.filtered_nodes;
    let index_of_interest = filtered_range.index_of_interest;

    let mut start_idx = 0;
    let mut end_idx = interested_nodes.len() - 1;
    let mut current_start_range = interested_nodes[start_idx];
    let mut current_end_range = interested_nodes[end_idx];
    while distance_between_nodes(&current_start_range, &current_end_range)
        > <i64>::try_from(line_limit).unwrap()
        && start_idx != end_idx
    {
        if index_of_interest - start_idx < end_idx - index_of_interest {
            end_idx = end_idx - 1;
            current_end_range = interested_nodes[end_idx];
        } else {
            start_idx = start_idx + 1;
            current_start_range = interested_nodes[start_idx];
        }
    }

    if distance_between_nodes(&current_start_range, &current_end_range)
        > <i64>::try_from(line_limit).unwrap()
    {
        Range::new(
            Position::from_tree_sitter_point(
                &current_start_range.start_position(),
                current_start_range.start_byte(),
            ),
            Position::from_tree_sitter_point(
                &current_end_range.end_position(),
                current_end_range.end_byte(),
            ),
        )
    } else {
        Range::for_tree_node(&some_other_node_to_name)
    }
}

fn distance_between_nodes(node: &tree_sitter::Node<'_>, other_node: &tree_sitter::Node<'_>) -> i64 {
    <i64>::try_from(other_node.end_position().row).unwrap()
        - <i64>::try_from(node.end_position().row).unwrap()
        + 1
}

fn keep_iterating<'a>(
    children: Vec<tree_sitter::Node<'a>>,
    current_node: tree_sitter::Node<'a>,
    language_config: &'a TSLanguageConfig,
    should_go_inside: bool,
) -> Option<FilteredRanges<'a>> {
    let mut filtered_children = vec![];
    let mut index = None;
    if should_go_inside {
        filtered_children = children
            .into_iter()
            .filter(|node| {
                language_config
                    .construct_types
                    .contains(&node.kind().to_owned())
                    || language_config
                        .expression_statements
                        .contains(&node.kind().to_owned())
            })
            .collect::<Vec<_>>();
        index = Some(binary_search(filtered_children.to_vec(), &current_node));
        filtered_children.insert(index.expect("binary search always returns"), current_node);
    } else {
        filtered_children = children
            .into_iter()
            .filter(|node| {
                language_config
                    .construct_types
                    .contains(&node.kind().to_owned())
                    || language_config
                        .expression_statements
                        .contains(&node.kind().to_owned())
                    || (node.start_byte() <= current_node.start_byte()
                        && node.end_byte() >= current_node.end_byte())
            })
            .collect::<Vec<_>>();
        index = filtered_children.to_vec().into_iter().position(|node| {
            node.start_byte() <= current_node.start_byte()
                && node.end_byte() >= current_node.end_byte()
        })
    }

    index.map(|index| FilteredRanges {
        filtered_nodes: filtered_children,
        index_of_interest: index,
    })
}

struct FilteredRanges<'a> {
    filtered_nodes: Vec<tree_sitter::Node<'a>>,
    index_of_interest: usize,
}

fn binary_search<'a>(
    nodes: Vec<tree_sitter::Node<'a>>,
    current_node: &tree_sitter::Node<'_>,
) -> usize {
    let mut start = 0;
    let mut end = nodes.len();

    while start < end {
        let mid = (start + end) / 2;
        if nodes[mid].range().start_byte < current_node.range().start_byte {
            start = mid + 1;
        } else {
            end = mid;
        }
    }
    start
}

fn get_merged_documentation_nodes(matches: Vec<tree_sitter::Node>, source: &str) -> Vec<String> {
    let mut comments = Vec::new();
    let mut current_index = 0;

    while current_index < matches.len() {
        let mut lines = Vec::new();
        lines.push(get_text_from_source(
            source,
            &matches[current_index].range(),
        ));

        while current_index + 1 < matches.len()
            && matches[current_index].range().end_point.row + 1
                == matches[current_index + 1].range().start_point.row
        {
            current_index += 1;
            lines.push(get_text_from_source(
                source,
                &matches[current_index].range(),
            ));
        }

        comments.push(lines.join("\n"));
        current_index += 1;
    }
    comments
}

fn get_text_from_source(source: &str, range: &tree_sitter::Range) -> String {
    source[range.start_byte..range.end_byte].to_owned()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub language: Option<String>,
    pub data: Option<String>,
}

impl Span {
    fn new(start: usize, end: usize, language: Option<String>, data: Option<String>) -> Self {
        Self {
            start,
            end,
            language,
            data,
        }
    }

    fn len(&self) -> usize {
        self.end - self.start
    }
}

fn chunk_node(
    mut node: tree_sitter::Node,
    language: &TSLanguageConfig,
    max_chars: usize,
) -> Vec<Span> {
    let mut chunks: Vec<Span> = vec![];
    let mut current_chunk = Span::new(
        node.start_byte(),
        node.start_byte(),
        language.get_language(),
        None,
    );
    let mut node_walker = node.walk();
    let current_node_children = node.children(&mut node_walker);
    for child in current_node_children {
        if child.end_byte() - child.start_byte() > max_chars {
            chunks.push(current_chunk.clone());
            current_chunk = Span::new(
                child.end_byte(),
                child.end_byte(),
                language.get_language(),
                None,
            );
            chunks.extend(chunk_node(child, language, max_chars));
        } else if child.end_byte() - child.start_byte() + current_chunk.len() > max_chars {
            chunks.push(current_chunk.clone());
            current_chunk = Span::new(
                child.start_byte(),
                child.end_byte(),
                language.get_language(),
                None,
            );
        } else {
            current_chunk.end = child.end_byte();
        }
    }
    chunks.push(current_chunk);
    chunks
}

/// We want to get back the non whitespace length of the string
fn non_whitespace_len(s: &str) -> usize {
    s.chars().filter(|c| !c.is_whitespace()).count()
}

fn get_line_number(byte_position: usize, split_lines: &[&str]) -> usize {
    let mut line_number = 0;
    let mut current_position = 0;
    for line in split_lines {
        if current_position + line.len() > byte_position {
            return line_number;
        }
        current_position += line.len();
        line_number += 1;
    }
    line_number
}

pub fn chunk_tree(
    tree: &tree_sitter::Tree,
    language: &TSLanguageConfig,
    max_characters_per_chunk: usize,
    coalesce: usize,
    buffer_content: &str,
) -> Vec<Span> {
    let mut chunks: Vec<Span> = vec![];
    let root_node = tree.root_node();
    let split_lines = buffer_content.lines().collect::<Vec<_>>();
    chunks = chunk_node(root_node, language, max_characters_per_chunk);

    if chunks.len() == 0 {
        return Default::default();
    }
    if chunks.len() < 2 {
        return vec![Span::new(
            0,
            get_line_number(chunks[0].end, split_lines.as_slice()),
            language.get_language(),
            Some(buffer_content.to_owned()),
        )];
    }
    for (prev, curr) in chunks.to_vec().iter_mut().zip(chunks.iter_mut().skip(1)) {
        prev.end = curr.start;
    }

    let mut new_chunks: Vec<Span> = Default::default();
    let mut current_chunk = Span::new(0, 0, language.get_language(), None);
    for chunk in chunks.iter() {
        current_chunk = Span::new(
            current_chunk.start,
            chunk.end,
            language.get_language(),
            None,
        );
        if non_whitespace_len(buffer_content[current_chunk.start..current_chunk.end].trim())
            > coalesce
        {
            new_chunks.push(current_chunk.clone());
            current_chunk = Span::new(chunk.end, chunk.end, language.get_language(), None);
        }
    }

    if current_chunk.len() > 0 {
        new_chunks.push(current_chunk.clone());
    }

    let mut line_chunks = new_chunks
        .iter()
        .map(|chunk| {
            let start_line = get_line_number(chunk.start, split_lines.as_slice());
            let end_line = get_line_number(chunk.end, split_lines.as_slice());
            Span::new(start_line, end_line, language.get_language(), None)
        })
        .filter(|span| span.len() > 0)
        .collect::<Vec<Span>>();

    if line_chunks.len() > 1 && line_chunks.last().unwrap().len() < coalesce {
        let chunks_len = line_chunks.len();
        let last_chunk = line_chunks.last().unwrap().clone();
        let prev_chunk = line_chunks.get_mut(chunks_len - 2).unwrap();
        prev_chunk.end = last_chunk.end;
        line_chunks.pop();
    }

    let split_buffer = buffer_content.split("\n").collect::<Vec<_>>();

    line_chunks
        .into_iter()
        .map(|line_chunk| {
            let data: String = split_buffer[line_chunk.start..line_chunk.end].join("\n");
            Span {
                start: line_chunk.start,
                end: line_chunk.end,
                language: line_chunk.language,
                data: Some(data),
            }
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use tree_sitter::Parser;
    use tree_sitter::TreeCursor;

    use crate::chunking::text_document::Position;
    use crate::chunking::text_document::Range;
    use crate::chunking::types::FunctionInformation;
    use crate::chunking::types::FunctionNodeType;

    use super::naive_chunker;
    use super::TSLanguageParsing;

    fn get_naive_chunking_test_string<'a>() -> &'a str {
        r#"
        # @axflow/models/azure-openai/chat

        Interface with [Azure-OpenAI's Chat Completions API](https://learn.microsoft.com/en-us/azure/ai-services/openai/reference) using this module.
        
        Note that this is very close to the vanilla openAI interface, with some subtle minor differences (the return types contain content filter results, see the `AzureOpenAIChatTypes.ContentFilterResults` type ).
        
        In addition, the streaming methods sometimes return objects with empty `choices` arrays. This is automatically handled if you use the `streamTokens()` method.
        
        ```ts
        import { AzureOpenAIChat } from '@axflow/models/azure-openai/chat';
        import type { AzureOpenAIChatTypes } from '@axflow/models/azure-openai/chat';
        ```
        
        ```ts
        declare class AzureOpenAIChat {
          static run: typeof run;
          static stream: typeof stream;
          static streamBytes: typeof streamBytes;
          static streamTokens: typeof streamTokens;
        }
        ```
        
        ## `run`
        
        ```ts
        /**
         * Run a chat completion against the Azure-openAI API.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns an Azure OpenAI chat completion. See Azure's documentation for /chat/completions
         */
        declare function run(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<AzureOpenAIChatTypes.Response>;
        ```
        
        ## `streamBytes`
        
        ```ts
        /**
         * Run a streaming chat completion against the Azure-openAI API. The resulting stream is the raw unmodified bytes from the API.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns A stream of bytes directly from the API.
         */
        declare function streamBytes(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<ReadableStream<Uint8Array>>;
        ```
        
        ## `stream`
        
        ```ts
        /**
         * Run a streaming chat completion against the Azure-openAI API. The resulting stream is the parsed stream data as JavaScript objects.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * Example object:
         * {"id":"chatcmpl-864d71dHehdlb2Vjq7WP5nHz10LRO","object":"chat.completion.chunk","created":1696458457,"model":"gpt-4","choices":[{"index":0,"finish_reason":null,"delta":{"content":" me"}}],"usage":null}
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns A stream of objects representing each chunk from the API.
         */
        declare function stream(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<ReadableStream<AzureOpenAIChatTypes.Chunk>>;
        ```
        
        ## `streamTokens`
        
        ```ts
        /**
         * Run a streaming chat completion against the Azure-openAI API. The resulting stream emits only the string tokens.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns A stream of tokens from the API.
         */
        declare function streamTokens(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<ReadableStream<string>>;
        ```        
        "#
    }

    #[test]
    fn test_naive_chunker() {
        // The test buffer has a total length of 128, with a chunk of size 30
        // and overlap of 15 we get 9 chunks, its easy maths. ceil(128/15) == 9
        let chunks = naive_chunker(get_naive_chunking_test_string(), 30, 15);
        assert_eq!(chunks.len(), 9);
    }

    #[test]
    fn test_documentation_parsing_rust() {
        let source_code = r#"
/// Some comment
/// Some other comment
fn blah_blah() {

}

/// something else
struct A {
    /// something over here
    pub a: string,
}
        "#;
        let tree_sitter_parsing = TSLanguageParsing::init();
        let documentation = tree_sitter_parsing.parse_documentation(source_code, "rust");
        assert_eq!(
            documentation,
            vec![
                "/// Some comment\n/// Some other comment",
                "/// something else",
                "/// something over here",
            ]
        );
    }

    #[test]
    fn test_documentation_parsing_rust_another() {
        let source_code = "/// Returns the default user ID as a `String`.\n///\n/// The default user ID is set to \"codestory\".\nfn default_user_id() -> String {\n    \"codestory\".to_owned()\n}";
        let tree_sitter_parsing = TSLanguageParsing::init();
        let documentation = tree_sitter_parsing.parse_documentation(source_code, "rust");
        assert_eq!(
            documentation,
            vec![
                "/// Returns the default user ID as a `String`.\n///\n/// The default user ID is set to \"codestory\".",
            ],
        );
    }

    #[test]
    fn test_documentation_parsing_typescript() {
        let source_code = r#"
        /**
         * Run a streaming chat completion against the Azure-openAI API. The resulting stream emits only the string tokens.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns A stream of tokens from the API.
         */
        declare function streamTokens(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<ReadableStream<string>>;
        "#;

        let tree_sitter_parsing = TSLanguageParsing::init();
        let documentation = tree_sitter_parsing.parse_documentation(source_code, "typescript");
        assert_eq!(
            documentation,
            vec![
    "/**\n         * Run a streaming chat completion against the Azure-openAI API. The resulting stream emits only the string tokens.\n         *\n         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions\n         *\n         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.\n         * @param options\n         * @param options.apiKey Azure API key.\n         * @param options.resourceName Azure resource name.\n         * @param options.deploymentId Azure deployment id.\n         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.\n         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n         * @param options.headers Optionally add additional HTTP headers to the request.\n         * @param options.signal An AbortSignal that can be used to abort the fetch request.\n         *\n         * @returns A stream of tokens from the API.\n         */",
            ],
        );
    }

    #[test]
    fn test_function_body_parsing_rust() {
        let source_code = r#"
/// Some comment
/// Some other comment
fn blah_blah() {

}

/// something else
struct A {
    /// something over here
    pub a: string,
}

impl A {
    fn something_else() -> Option<String> {
        None
    }
}
        "#;

        let tree_sitter_parsing = TSLanguageParsing::init();
        let function_nodes = tree_sitter_parsing.function_information_nodes(source_code, "rust");

        // we should get back 2 function nodes here and since we capture 3 pieces
        // of information for each function block, in total that is 6
        assert_eq!(function_nodes.len(), 6);
    }

    #[test]
    fn test_fix_range_for_typescript() {
        let source_code = "import { POST, HttpError } from '@axflow/models/shared';\nimport { headers } from './shared';\nimport type { SharedRequestOptions } from './shared';\n\nconst COHERE_API_URL = 'https://api.cohere.ai/v1/generate';\n\nexport namespace CohereGenerationTypes {\n  export type Request = {\n    prompt: string;\n    model?: string;\n    num_generations?: number;\n    max_tokens?: number;\n    truncate?: string;\n    temperature?: number;\n    preset?: string;\n    end_sequences?: string[];\n    stop_sequences?: string[];\n    k?: number;\n    p?: number;\n    frequency_penalty?: number;\n    presence_penalty?: number;\n    return_likelihoods?: string;\n    logit_bias?: Record<string, any>;\n  };\n\n  export type RequestOptions = SharedRequestOptions;\n\n  export type Generation = {\n    id: string;\n    text: string;\n    index?: number;\n    likelihood?: number;\n    token_likelihoods?: Array<{\n      token: string;\n      likelihood: number;\n    }>;\n  };\n\n  export type Response = {\n    id: string;\n    prompt?: string;\n    generations: Generation[];\n    meta: {\n      api_version: {\n        version: string;\n        is_deprecated?: boolean;\n        is_experimental?: boolean;\n      };\n      warnings?: string[];\n    };\n  };\n\n  export type Chunk = {\n    text?: string;\n    is_finished: boolean;\n    finished_reason?: 'COMPLETE' | 'MAX_TOKENS' | 'ERROR' | 'ERROR_TOXIC';\n    response?: {\n      id: string;\n      prompt?: string;\n      generations: Generation[];\n    };\n  };\n}\n\n/**\n * Run a generation against the Cohere API.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns Cohere completion. See Cohere's documentation for /v1/generate.\n */\nasync function run(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<CohereGenerationTypes.Response> {\n  const url = options.apiUrl || COHERE_API_URL;\n\n  const response = await POST(url, {\n    headers: headers(options.apiKey, options.headers),\n    body: JSON.stringify({ ...request, stream: false }),\n    fetch: options.fetch,\n    signal: options.signal,\n  });\n\n  return response.json();\n}\n\n/**\n * Run a streaming generation against the Cohere API. The resulting stream is the raw unmodified bytes from the API.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns A stream of bytes directly from the API.\n */\nasync function streamBytes(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<ReadableStream<Uint8Array>> {\n  const url = options.apiUrl || COHERE_API_URL;\n\n  const response = await POST(url, {\n    headers: headers(options.apiKey, options.headers),\n    body: JSON.stringify({ ...request, stream: true }),\n    fetch: options.fetch,\n    signal: options.signal,\n  });\n\n  if (!response.body) {\n    throw new HttpError('Expected response body to be a ReadableStream', response);\n  }\n\n  return response.body;\n}\n\nfunction noop(chunk: CohereGenerationTypes.Chunk) {\n  return chunk;\n}\n\n/**\n * Run a streaming generation against the Cohere API. The resulting stream is the parsed stream data as JavaScript objects.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns A stream of objects representing each chunk from the API.\n */\nasync function stream(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<ReadableStream<CohereGenerationTypes.Chunk>> {\n  const byteStream = await streamBytes(request, options);\n  return byteStream.pipeThrough(new CohereGenerationDecoderStream(noop));\n}\n\nfunction chunkToToken(chunk: CohereGenerationTypes.Chunk) {\n  return chunk.text || '';\n}\n\n/**\n * Run a streaming generation against the Cohere API. The resulting stream emits only the string tokens.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns A stream of tokens from the API.\n */\nasync function streamTokens(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<ReadableStream<string>> {\n  const byteStream = await streamBytes(request, options);\n  return byteStream.pipeThrough(new CohereGenerationDecoderStream(chunkToToken));\n}\n\n/**\n * An object that encapsulates methods for calling the Cohere Generate API.\n */\nexport class CohereGeneration {\n  static run = run;\n  static stream = stream;\n  static streamBytes = streamBytes;\n  static streamTokens = streamTokens;\n}\n\nclass CohereGenerationDecoderStream<T> extends TransformStream<Uint8Array, T> {\n  private static parse(line: string): CohereGenerationTypes.Chunk | null {\n    line = line.trim();\n\n    // Empty lines are ignored\n    if (line.length === 0) {\n      return null;\n    }\n\n    try {\n      return JSON.parse(line);\n    } catch (error) {\n      throw new Error(\n        `Invalid event: expected well-formed event lines but got ${JSON.stringify(line)}`,\n      );\n    }\n  }\n\n  private static transformer<T>(map: (chunk: CohereGenerationTypes.Chunk) => T) {\n    let buffer: string[] = [];\n    const decoder = new TextDecoder();\n\n    return (bytes: Uint8Array, controller: TransformStreamDefaultController<T>) => {\n      const chunk = decoder.decode(bytes);\n\n      for (let i = 0, len = chunk.length; i < len; ++i) {\n        // Cohere separates events with '\\n'\n        const isEventSeparator = chunk[\"something\"] === '\\n';\n\n        // Keep buffering unless we've hit the end of an event\n        if (!isEventSeparator) {\n          buffer.push(chunk[i]);\n          continue;\n        }\n\n        const event = CohereGenerationDecoderStream.parse(buffer.join(''));\n\n        if (event) {\n          controller.enqueue(map(event));\n        }\n\n        buffer = [];\n      }\n    };\n  }\n\n  constructor(map: (chunk: CohereGenerationTypes.Chunk) => T) {\n    super({ transform: CohereGenerationDecoderStream.transformer(map) });\n  }\n}\n";
        let language = "typescript";
        let range = Range::new(Position::new(217, 45, 7441), Position::new(217, 45, 7441));
        let extra_width = 15;
        let tree_sitter_parsing = TSLanguageParsing::init();
        let fix_range =
            tree_sitter_parsing.get_fix_range(source_code, language, &range, extra_width);
        assert!(fix_range.is_some());
        let fix_range = fix_range.expect("is_some to work");
        let generated_range = source_code[fix_range.start_byte()..fix_range.end_byte()].to_owned();
        assert_eq!(generated_range, "{\n        // Cohere separates events with '\\n'\n        const isEventSeparator = chunk[\"something\"] === '\\n';\n\n        // Keep buffering unless we've hit the end of an event\n        if (!isEventSeparator) {\n          buffer.push(chunk[i]);\n          continue;\n        }\n\n        const event = CohereGenerationDecoderStream.parse(buffer.join(''));\n\n        if (event) {\n          controller.enqueue(map(event));\n        }\n\n        buffer = [];\n      }");
    }

    #[test]
    fn test_function_nodes_for_typescript() {
        let source_code = "import { POST, HttpError } from '@axflow/models/shared';\nimport { headers } from './shared';\nimport type { SharedRequestOptions } from './shared';\n\nconst COHERE_API_URL = 'https://api.cohere.ai/v1/generate';\n\nexport namespace CohereGenerationTypes {\n  export type Request = {\n    prompt: string;\n    model?: string;\n    num_generations?: number;\n    max_tokens?: number;\n    truncate?: string;\n    temperature?: number;\n    preset?: string;\n    end_sequences?: string[];\n    stop_sequences?: string[];\n    k?: number;\n    p?: number;\n    frequency_penalty?: number;\n    presence_penalty?: number;\n    return_likelihoods?: string;\n    logit_bias?: Record<string, any>;\n  };\n\n  export type RequestOptions = SharedRequestOptions;\n\n  export type Generation = {\n    id: string;\n    text: string;\n    index?: number;\n    likelihood?: number;\n    token_likelihoods?: Array<{\n      token: string;\n      likelihood: number;\n    }>;\n  };\n\n  export type Response = {\n    id: string;\n    prompt?: string;\n    generations: Generation[];\n    meta: {\n      api_version: {\n        version: string;\n        is_deprecated?: boolean;\n        is_experimental?: boolean;\n      };\n      warnings?: string[];\n    };\n  };\n\n  export type Chunk = {\n    text?: string;\n    is_finished: boolean;\n    finished_reason?: 'COMPLETE' | 'MAX_TOKENS' | 'ERROR' | 'ERROR_TOXIC';\n    response?: {\n      id: string;\n      prompt?: string;\n      generations: Generation[];\n    };\n  };\n}\n\n/**\n * Run a generation against the Cohere API.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns Cohere completion. See Cohere's documentation for /v1/generate.\n */\nasync function run(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<CohereGenerationTypes.Response> {\n  const url = options.apiUrl || COHERE_API_URL;\n\n  const response = await POST(url, {\n    headers: headers(options.apiKey, options.headers),\n    body: JSON.stringify({ ...request, stream: false }),\n    fetch: options.fetch,\n    signal: options.signal,\n  });\n\n  return response.json();\n}\n\n/**\n * Run a streaming generation against the Cohere API. The resulting stream is the raw unmodified bytes from the API.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns A stream of bytes directly from the API.\n */\nasync function streamBytes(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<ReadableStream<Uint8Array>> {\n  const url = options.apiUrl || COHERE_API_URL;\n\n  const response = await POST(url, {\n    headers: headers(options.apiKey, options.headers),\n    body: JSON.stringify({ ...request, stream: true }),\n    fetch: options.fetch,\n    signal: options.signal,\n  });\n\n  if (!response.body) {\n    throw new HttpError('Expected response body to be a ReadableStream', response);\n  }\n\n  return response.body;\n}\n\nfunction noop(chunk: CohereGenerationTypes.Chunk) {\n  return chunk;\n}\n\n/**\n * Run a streaming generation against the Cohere API. The resulting stream is the parsed stream data as JavaScript objects.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns A stream of objects representing each chunk from the API.\n */\nasync function stream(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<ReadableStream<CohereGenerationTypes.Chunk>> {\n  const byteStream = await streamBytes(request, options);\n  return byteStream.pipeThrough(new CohereGenerationDecoderStream(noop));\n}\n\nfunction chunkToToken(chunk: CohereGenerationTypes.Chunk) {\n  return chunk.text || '';\n}\n\n/**\n * Run a streaming generation against the Cohere API. The resulting stream emits only the string tokens.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns A stream of tokens from the API.\n */\nasync function streamTokens(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<ReadableStream<string>> {\n  const byteStream = await streamBytes(request, options);\n  return byteStream.pipeThrough(new CohereGenerationDecoderStream(chunkToToken));\n}\n\n/**\n * An object that encapsulates methods for calling the Cohere Generate API.\n */\nexport class CohereGeneration {\n  static run = run;\n  static stream = stream;\n  static streamBytes = streamBytes;\n  static streamTokens = streamTokens;\n}\n\nclass CohereGenerationDecoderStream<T> extends TransformStream<Uint8Array, T> {\n  private static parse(line: string): CohereGenerationTypes.Chunk | null {\n    line = line.trim();\n\n    // Empty lines are ignored\n    if (line.length === 0) {\n      return null;\n    }\n\n    try {\n      return JSON.parse(line);\n    } catch (error) {\n      throw new Error(\n        `Invalid event: expected well-formed event lines but got ${JSON.stringify(line)}`,\n      );\n    }\n  }\n\n  private static transformer<T>(map: (chunk: CohereGenerationTypes.Chunk) => T) {\n    let buffer: string[] = [];\n    const decoder = new TextDecoder();\n\n    return (bytes: Uint8Array, controller: TransformStreamDefaultController<T>) => {\n      const chunk = decoder.decode(bytes);\n\n      for (let i = 0, len = chunk.length; i < len; ++i) {\n        // Cohere separates events with '\\n'\n        const isEventSeparator = chunk[\"something\"] === '\\n';\n\n        // Keep buffering unless we've hit the end of an event\n        if (!isEventSeparator) {\n          buffer.push(chunk[i]);\n          continue;\n        }\n\n        const event = CohereGenerationDecoderStream.parse(buffer.join(''));\n\n        if (event) {\n          controller.enqueue(map(event));\n        }\n\n        buffer = [];\n      }\n    };\n  }\n\n  constructor(map: (chunk: CohereGenerationTypes.Chunk) => T) {\n    super({ transform: CohereGenerationDecoderStream.transformer(map) });\n  }\n}\n";
        let language = "typescript";
        let tree_sitter_parsing = TSLanguageParsing::init();
        let ts_language_config = tree_sitter_parsing
            .for_lang(language)
            .expect("test to work");
        let function_data = ts_language_config.capture_function_data(source_code.as_bytes());
        assert!(true);
    }

    #[test]
    fn test_type_nodes_for_typescript() {
        let source_code = r#"
// Some random comment over here
type SometingElse = {
    a: string,
    b: number,
};

// something else over here as comment
export type SomethingInterface = {
    a: string,
    b: number,
};

namespace SomeNamespace {
    export type Something = {
        a: string,
        b: number,
    };
}
        "#;
        let language = "typescript";
        let tree_sitter_parsing = TSLanguageParsing::init();
        let ts_language_config = tree_sitter_parsing
            .for_lang(language)
            .expect("test to work");
        let type_information = ts_language_config.capture_type_data(source_code.as_bytes());
        assert_eq!(type_information.len(), 3);
        assert_eq!(type_information[0].name, "SometingElse");
        assert_eq!(
            type_information[0].documentation,
            Some("// Some random comment over here".to_owned())
        );
        assert_eq!(type_information[1].name, "SomethingInterface");
    }

    #[test]
    fn test_function_nodes_documentation_for_typescript() {
        let source_code = r#"
// Updates an existing chat agent with new metadata
function updateAgent(id: string, updateMetadata: ICSChatAgentMetadata): void {
    // ...
}

// Returns the default chat agent
function getDefaultAgent(): IChatAgent | undefined {
    // ...
}

// Returns the secondary chat agent
function getSecondaryAgent(): IChatAgent | undefined {
    // ...
}

// Returns all registered chat agents
function getAgents(): Array<IChatAgent> {
    // ...
}

// Checks if a chat agent with the given id exists
function hasAgent(id: string): boolean {
    // ...
}

// Returns a chat agent with the given id
function getAgent(id: string): IChatAgent | undefined {
    // ...
}

// Invokes a chat agent with the given id and request
async function invokeAgent(id: string, request: ICSChatAgentRequest, progress: (part: ICSChatProgress) => void, history: ICSChatMessage[], token: CancellationToken): Promise<ICSChatAgentResult> {
    // ...
}

// Returns followups for a chat agent with the given id and session id
async getFollowups(id: string, sessionId: string, token: CancellationToken): Promise<ICSChatFollowup[]> {
    // ...
}

// Returns edits for a chat agent with the given context
async function getEdits(context: ICSChatAgentEditRequest, progress: (part: ICSChatAgentEditRepsonse) => void, token: CancellationToken): Promise<ICSChatAgentEditRepsonse | undefined> {
    // ...
}"#;
        let tree_sitter_parsing = TSLanguageParsing::init();
        let ts_language_config = tree_sitter_parsing.for_lang("typescript").expect("to work");
        let fn_info = ts_language_config.capture_function_data(source_code.as_bytes());
        assert!(true);
    }

    #[test]
    fn test_outline_for_typescript() {
        let source_code = "import { POST, HttpError } from '@axflow/models/shared';\nimport { headers } from './shared';\nimport type { SharedRequestOptions } from './shared';\n\nconst COHERE_API_URL = 'https://api.cohere.ai/v1/generate';\n\nexport namespace CohereGenerationTypes {\n  export type Request = {\n    prompt: string;\n    model?: string;\n    num_generations?: number;\n    max_tokens?: number;\n    truncate?: string;\n    temperature?: number;\n    preset?: string;\n    end_sequences?: string[];\n    stop_sequences?: string[];\n    k?: number;\n    p?: number;\n    frequency_penalty?: number;\n    presence_penalty?: number;\n    return_likelihoods?: string;\n    logit_bias?: Record<string, any>;\n  };\n\n  export type RequestOptions = SharedRequestOptions;\n\n  export type Generation = {\n    id: string;\n    text: string;\n    index?: number;\n    likelihood?: number;\n    token_likelihoods?: Array<{\n      token: string;\n      likelihood: number;\n    }>;\n  };\n\n  export type Response = {\n    id: string;\n    prompt?: string;\n    generations: Generation[];\n    meta: {\n      api_version: {\n        version: string;\n        is_deprecated?: boolean;\n        is_experimental?: boolean;\n      };\n      warnings?: string[];\n    };\n  };\n\n  export type Chunk = {\n    text?: string;\n    is_finished: boolean;\n    finished_reason?: 'COMPLETE' | 'MAX_TOKENS' | 'ERROR' | 'ERROR_TOXIC';\n    response?: {\n      id: string;\n      prompt?: string;\n      generations: Generation[];\n    };\n  };\n}\n\n/**\n * Run a generation against the Cohere API.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns Cohere completion. See Cohere's documentation for /v1/generate.\n */\nasync function run(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<CohereGenerationTypes.Response> {\n  const url = options.apiUrl || COHERE_API_URL;\n\n  const response = await POST(url, {\n    headers: headers(options.apiKey, options.headers),\n    body: JSON.stringify({ ...request, stream: false }),\n    fetch: options.fetch,\n    signal: options.signal,\n  });\n\n  return response.json();\n}\n\n/**\n * Run a streaming generation against the Cohere API. The resulting stream is the raw unmodified bytes from the API.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns A stream of bytes directly from the API.\n */\nasync function streamBytes(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<ReadableStream<Uint8Array>> {\n  const url = options.apiUrl || COHERE_API_URL;\n\n  const response = await POST(url, {\n    headers: headers(options.apiKey, options.headers),\n    body: JSON.stringify({ ...request, stream: true }),\n    fetch: options.fetch,\n    signal: options.signal,\n  });\n\n  if (!response.body) {\n    throw new HttpError('Expected response body to be a ReadableStream', response);\n  }\n\n  return response.body;\n}\n\nfunction noop(chunk: CohereGenerationTypes.Chunk) {\n  return chunk;\n}\n\n/**\n * Run a streaming generation against the Cohere API. The resulting stream is the parsed stream data as JavaScript objects.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns A stream of objects representing each chunk from the API.\n */\nasync function stream(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<ReadableStream<CohereGenerationTypes.Chunk>> {\n  const byteStream = await streamBytes(request, options);\n  return byteStream.pipeThrough(new CohereGenerationDecoderStream(noop));\n}\n\nfunction chunkToToken(chunk: CohereGenerationTypes.Chunk) {\n  return chunk.text || '';\n}\n\n/**\n * Run a streaming generation against the Cohere API. The resulting stream emits only the string tokens.\n *\n * @see https://docs.cohere.com/reference/generate\n *\n * @param request The request body sent to Cohere. See Cohere's documentation for /v1/generate for supported parameters.\n * @param options\n * @param options.apiKey Cohere API key.\n * @param options.apiUrl The url of the Cohere (or compatible) API. Defaults to https://api.cohere.ai/v1/generate.\n * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n * @param options.headers Optionally add additional HTTP headers to the request.\n * @param options.signal An AbortSignal that can be used to abort the fetch request.\n * @returns A stream of tokens from the API.\n */\nasync function streamTokens(\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n): Promise<ReadableStream<string>> {\n  const byteStream = await streamBytes(request, options);\n  return byteStream.pipeThrough(new CohereGenerationDecoderStream(chunkToToken));\n}\n\n/**\n * An object that encapsulates methods for calling the Cohere Generate API.\n */\nexport class CohereGeneration {\n  static run = run;\n  static stream = stream;\n  static streamBytes = streamBytes;\n  static streamTokens = streamTokens;\n}\n\nclass CohereGenerationDecoderStream<T> extends TransformStream<Uint8Array, T> {\n  private static parse(line: string): CohereGenerationTypes.Chunk | null {\n    line = line.trim();\n\n    // Empty lines are ignored\n    if (line.length === 0) {\n      return null;\n    }\n\n    try {\n      return JSON.parse(line);\n    } catch (error) {\n      throw new Error(\n        `Invalid event: expected well-formed event lines but got ${JSON.stringify(line)}`,\n      );\n    }\n  }\n\n  private static transformer<T>(map: (chunk: CohereGenerationTypes.Chunk) => T) {\n    let buffer: string[] = [];\n    const decoder = new TextDecoder();\n\n    return (bytes: Uint8Array, controller: TransformStreamDefaultController<T>) => {\n      const chunk = decoder.decode(bytes);\n\n      for (let i = 0, len = chunk.length; i < len; ++i) {\n        // Cohere separates events with '\\n'\n        const isEventSeparator = chunk[\"something\"] === '\\n';\n\n        // Keep buffering unless we've hit the end of an event\n        if (!isEventSeparator) {\n          buffer.push(chunk[i]);\n          continue;\n        }\n\n        const event = CohereGenerationDecoderStream.parse(buffer.join(''));\n\n        if (event) {\n          controller.enqueue(map(event));\n        }\n\n        buffer = [];\n      }\n    };\n  }\n\n  constructor(map: (chunk: CohereGenerationTypes.Chunk) => T) {\n    super({ transform: CohereGenerationDecoderStream.transformer(map) });\n  }\n}\n";
        let language = "typescript";
        let tree_sitter_parsing = TSLanguageParsing::init();
        let ts_language_config = tree_sitter_parsing
            .for_lang(language)
            .expect("test to work");
        ts_language_config.capture_class_data(source_code.as_bytes());
        let outline = ts_language_config.generate_file_outline_str(source_code.as_bytes());
        assert_eq!(outline, "```typescript\n\nClass CohereGeneration\n\nClass CohereGenerationDecoderStream\n\n    function parse((line: string)): : CohereGenerationTypes.Chunk | null\n    function transformer((map: (chunk: CohereGenerationTypes.Chunk) => T)): \n    function constructor((map: (chunk: CohereGenerationTypes.Chunk) => T)): \nfunction run((\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n)): : Promise<CohereGenerationTypes.Response>\n\nfunction streamBytes((\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n)): : Promise<ReadableStream<Uint8Array>>\n\nfunction noop((chunk: CohereGenerationTypes.Chunk)): \n\nfunction stream((\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n)): : Promise<ReadableStream<CohereGenerationTypes.Chunk>>\n\nfunction chunkToToken((chunk: CohereGenerationTypes.Chunk)): \n\nfunction streamTokens((\n  request: CohereGenerationTypes.Request,\n  options: CohereGenerationTypes.RequestOptions,\n)): : Promise<ReadableStream<string>>\n\n```");
    }

    fn walk(cursor: &mut TreeCursor, indent: usize) {
        loop {
            let node = cursor.node();
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            println!(
                "{}{:?}({}:{}): error:{} missing:{}",
                " ".repeat(indent),
                node.kind(),
                start_byte,
                end_byte,
                node.is_error(),
                // TODO(skcd): Found it! We can use this to determine if there are
                // any linter errors and then truncate using this, until we do not introduce
                // any more errors
                node.is_missing(),
            );

            if cursor.goto_first_child() {
                walk(cursor, indent + 2);
                cursor.goto_parent();
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    #[test]
    fn test_typescript_error_parsing() {
        let source_code = r#"
function add(a: number, b: number): number {
    !!!!!!!
    return a + b;
}
"#;
        let language = "typescript";
        let tree_sitter_parsing = TSLanguageParsing::init();
        let ts_language_config = tree_sitter_parsing
            .for_lang(language)
            .expect("test to work");
        let grammar = ts_language_config.grammar;
        let mut parser = Parser::new();
        parser.set_language(grammar()).unwrap();
        let tree = parser.parse(source_code.as_bytes(), None).unwrap();
        let mut visitors = tree.walk();
        walk(&mut visitors, 0);
        assert!(false);
    }

    fn walk_tree_for_no_errors(
        cursor: &mut TreeCursor,
        inserted_range: &Range,
        indent: usize,
    ) -> bool {
        let mut answer = true;
        loop {
            let node = cursor.node();
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();

            fn check_if_inside_range(
                start_byte: usize,
                end_byte: usize,
                inserted_byte: usize,
            ) -> bool {
                start_byte <= inserted_byte && inserted_byte <= end_byte
            }

            // TODO(skcd): Check this condition here tomorrow so we can handle cases
            // where the missing shows up at the end of the node, because that ends up
            // happening quite often
            fn check_if_intersects_range(
                start_byte: usize,
                end_byte: usize,
                inserted_range: &Range,
            ) -> bool {
                check_if_inside_range(start_byte, end_byte, inserted_range.start_byte())
                    || check_if_inside_range(start_byte, end_byte, inserted_range.end_byte())
            }

            println!(
                "{}{:?}({}:{}): error:{} missing:{} does_intersect({}:{}): {}",
                " ".repeat(indent),
                node.kind(),
                start_byte,
                end_byte,
                node.is_error(),
                // TODO(skcd): Found it! We can use this to determine if there are
                // any linter errors and then truncate using this, until we do not introduce
                // any more errors
                node.is_missing(),
                inserted_range.start_byte(),
                inserted_range.end_byte(),
                check_if_intersects_range(start_byte, end_byte, inserted_range),
            );

            // First check if the node is in the range or
            // the range of the node intersects with the inserted range
            if check_if_intersects_range(
                node.range().start_byte,
                node.range().end_byte,
                inserted_range,
            ) {
                if node.is_error() || node.is_missing() {
                    answer = false;
                    return answer;
                }
            }

            if cursor.goto_first_child() {
                answer = answer && walk_tree_for_no_errors(cursor, inserted_range, indent + 1);
                if !answer {
                    return answer;
                }
                cursor.goto_parent();
            }

            if !cursor.goto_next_sibling() {
                return answer;
            }
        }
    }

    #[test]
    fn test_rust_error_checking() {
        let source_code = r#"use sidecar::{embedder::embedder::Embedder, embedder::embedder::LocalEmbedder};
use std::env;

#[tokio::main]
async fn main() {
    println!("Hello, world! skcd");
    init_ort_dylib();

    // Now we try to create the embedder and see if thats working
    let current_path = env::current_dir().unwrap();
    // Checking that the embedding logic is also working
    let embedder = LocalEmbedder::new(&current_path.join("models/all-MiniLM-L6-v2/")).unwrap();
    let result = embedder.embed("hello world!").unwrap();
    dbg!(result.len());
    dbg!(result);
}

fn add(left:)

fn init_ort_dylib() {
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "linux")]
        let lib_path = "libonnxruntime.so";
        #[cfg(target_os = "macos")]
        let lib_path =
            "/Users/skcd/Downloads/onnxruntime-osx-arm64-1.16.0/lib/libonnxruntime.dylib";

        // let ort_dylib_path = dylib_dir.as_ref().join(lib_name);

        if env::var("ORT_DYLIB_PATH").is_err() {
            env::set_var("ORT_DYLIB_PATH", lib_path);
        }
    }
}"#;
        let language = "rust";
        let tree_sitter_parsing = TSLanguageParsing::init();
        let ts_language_config = tree_sitter_parsing
            .for_lang(language)
            .expect("test to work");
        let grammar = ts_language_config.grammar;
        let mut parser = Parser::new();
        parser.set_language(grammar()).unwrap();
        // the range we are checking is this:
        // let range = Range {
        //     start_position: Position {
        //         line: 17,
        //         character: 7,
        //         byte_offset: 568,
        //     },
        //     end_position: Position {
        //         line: 17,
        //         character: 13,
        //         byte_offset: 574,
        //     },
        // };
        let range = Range::new(Position::new(17, 7, 568), Position::new(17, 7, 574));
        let tree = parser.parse(source_code.as_bytes(), None).unwrap();
        let mut visitors = tree.walk();
        // walk(&mut visitors, 0);
        dbg!(walk_tree_for_no_errors(&mut visitors, &range, 0));
        assert!(false);
    }

    #[test]
    fn test_typescript_error_checking() {
        let source_code = r#"class A {
public somethingelse() {}
public something() {
}"#;
        let language = "typescript";
        let tree_sitter_parsing = TSLanguageParsing::init();
        let ts_language_config = tree_sitter_parsing
            .for_lang(language)
            .expect("test to work");
        let grammar = ts_language_config.grammar;
        let mut parser = Parser::new();
        parser.set_language(grammar()).unwrap();
        let tree = parser.parse(source_code.as_bytes(), None).unwrap();
        let mut visitors = tree.walk();
        walk(&mut visitors, 0);
        assert!(false);
    }
}
