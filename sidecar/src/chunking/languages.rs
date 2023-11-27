use std::collections::HashSet;

use crate::chunking::types::FunctionNodeInformation;

use super::{
    javascript::javascript_language_config,
    python::python_language_config,
    rust::rust_language_config,
    text_document::{Position, Range},
    types::{
        ClassInformation, ClassNodeType, ClassWithFunctions, FunctionInformation, FunctionNodeType,
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
    pub namespaces: Vec<String>,

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
        compressed_classes
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
        FunctionInformation::fold_function_blocks(compressed_functions)
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
    let split_lines = buffer_content.split("\n").collect::<Vec<_>>();
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
    fn test_function_nodes_documentation_for_typescript() {
        let source_code = r#"
    // Registers a new chat agent
    /**
     * Soething over here
     */
    function registerAgent(agent: IChatAgent): IDisposable {
        // ...
    }

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
        assert!(false);
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

    #[test]
    fn test_outline_for_typescript_large() {
        let source_code = r#"
        /*---------------------------------------------------------------------------------------------
        *  Copyright (c) Microsoft Corporation. All rights reserved.
        *  Licensed under the MIT License. See License.txt in the project root for license information.
        *--------------------------------------------------------------------------------------------*/
       
       import * as aria from 'vs/base/browser/ui/aria/aria';
       import { Barrier, Queue, raceCancellation, raceCancellationError } from 'vs/base/common/async';
       import { CancellationTokenSource } from 'vs/base/common/cancellation';
       import { toErrorMessage } from 'vs/base/common/errorMessage';
       import { Emitter, Event } from 'vs/base/common/event';
       import { DisposableStore, IDisposable, MutableDisposable, toDisposable } from 'vs/base/common/lifecycle';
       import { StopWatch } from 'vs/base/common/stopwatch';
       import { assertType } from 'vs/base/common/types';
       import { ICodeEditor } from 'vs/editor/browser/editorBrowser';
       import { IPosition, Position } from 'vs/editor/common/core/position';
       import { IRange, Range } from 'vs/editor/common/core/range';
       import { IEditorContribution } from 'vs/editor/common/editorCommon';
       import { IEditorWorkerService } from 'vs/editor/common/services/editorWorker';
       import { InlineCompletionsController } from 'vs/editor/contrib/inlineCompletions/browser/inlineCompletionsController';
       import { localize } from 'vs/nls';
       import { IAccessibilityService } from 'vs/platform/accessibility/common/accessibility';
       import { IConfigurationService } from 'vs/platform/configuration/common/configuration';
       import { IContextKey, IContextKeyService } from 'vs/platform/contextkey/common/contextkey';
       import { IDialogService } from 'vs/platform/dialogs/common/dialogs';
       import { IInstantiationService, ServicesAccessor } from 'vs/platform/instantiation/common/instantiation';
       import { ILogService } from 'vs/platform/log/common/log';
       import { EmptyResponse, ErrorResponse, ExpansionState, IInlineChatSessionService, ReplyResponse, Session, SessionExchange, SessionPrompt } from 'vs/workbench/contrib/inlineCSChat/browser/inlineCSChatSession';
       import { EditModeStrategy, LivePreviewStrategy, LiveStrategy, PreviewStrategy, ProgressingEditsOptions } from 'vs/workbench/contrib/inlineCSChat/browser/inlineCSChatStrategies';
       import { IInlineChatMessageAppender, InlineChatWidget, InlineChatZoneWidget } from 'vs/workbench/contrib/inlineCSChat/browser/inlineCSChatWidget';
       import { CTX_INLINE_CHAT_HAS_ACTIVE_REQUEST, CTX_INLINE_CHAT_LAST_FEEDBACK, IInlineCSChatRequest, IInlineCSChatResponse, INLINE_CHAT_ID, EditMode, InlineCSChatResponseFeedbackKind, CTX_INLINE_CHAT_LAST_RESPONSE_TYPE, InlineChatResponseType, CTX_INLINE_CHAT_DID_EDIT, CTX_INLINE_CHAT_HAS_STASHED_SESSION, InlineChateResponseTypes, CTX_INLINE_CHAT_RESPONSE_TYPES, CTX_INLINE_CHAT_USER_DID_EDIT, IInlineCSChatProgressItem, CTX_INLINE_CHAT_SUPPORT_ISSUE_REPORTING } from 'vs/workbench/contrib/inlineCSChat/common/inlineCSChat';
       import { ICSChatAccessibilityService, ICSChatWidgetService } from 'vs/workbench/contrib/csChat/browser/csChat';
       import { ICSChatService } from 'vs/workbench/contrib/csChat/common/csChatService';
       import { IKeybindingService } from 'vs/platform/keybinding/common/keybinding';
       import { Lazy } from 'vs/base/common/lazy';
       import { Progress } from 'vs/platform/progress/common/progress';
       import { generateUuid } from 'vs/base/common/uuid';
       import { TextEdit } from 'vs/editor/common/languages';
       import { ISelection, Selection } from 'vs/editor/common/core/selection';
       import { onUnexpectedError } from 'vs/base/common/errors';
       import { MarkdownString } from 'vs/base/common/htmlContent';
       import { MovingAverage } from 'vs/base/common/numbers';
       import { ModelDecorationOptions } from 'vs/editor/common/model/textModel';
       import { IModelDeltaDecoration } from 'vs/editor/common/model';
       import { ICSChatAgentService } from 'vs/workbench/contrib/csChat/common/csChatAgents';
       import { chatAgentLeader, chatSubcommandLeader } from 'vs/workbench/contrib/csChat/common/csChatParserTypes';
       import { renderMarkdownAsPlaintext } from 'vs/base/browser/markdownRenderer';
       import { IInlineCSChatVariablesService } from 'vs/workbench/contrib/csChat/common/csChatVariables';
       import { InlineCSChatRequestParser } from 'vs/workbench/contrib/inlineCSChat/common/inlineCSChatRequestParser';
       import { IBulkEditService } from 'vs/editor/browser/services/bulkEditService';
       import { IStorageService, StorageScope, StorageTarget } from 'vs/platform/storage/common/storage';
       
       export const enum State {
           CREATE_SESSION = 'CREATE_SESSION',
           INIT_UI = 'INIT_UI',
           WAIT_FOR_INPUT = 'WAIT_FOR_INPUT',
           MAKE_REQUEST = 'MAKE_REQUEST',
           APPLY_RESPONSE = 'APPLY_RESPONSE',
           SHOW_RESPONSE = 'SHOW_RESPONSE',
           PAUSE = 'PAUSE',
           CANCEL = 'CANCEL',
           ACCEPT = 'DONE',
       }
       
       const enum Message {
           NONE = 0,
           ACCEPT_SESSION = 1 << 0,
           CANCEL_SESSION = 1 << 1,
           PAUSE_SESSION = 1 << 2,
           CANCEL_REQUEST = 1 << 3,
           CANCEL_INPUT = 1 << 4,
           ACCEPT_INPUT = 1 << 5,
           RERUN_INPUT = 1 << 6,
       }
       
       export abstract class InlineChatRunOptions {
           initialSelection?: ISelection;
           initialRange?: IRange;
           message?: string;
           autoSend?: boolean;
           existingSession?: Session;
           isUnstashed?: boolean;
           position?: IPosition;
       
           static isInteractiveEditorOptions(options: any): options is InlineChatRunOptions {
               const { initialSelection, initialRange, message, autoSend, position } = options;
               if (
                   typeof message !== 'undefined' && typeof message !== 'string'
                   || typeof autoSend !== 'undefined' && typeof autoSend !== 'boolean'
                   || typeof initialRange !== 'undefined' && !Range.isIRange(initialRange)
                   || typeof initialSelection !== 'undefined' && !Selection.isISelection(initialSelection)
                   || typeof position !== 'undefined' && !Position.isIPosition(position)) {
                   return false;
               }
               return true;
           }
       }
       
       export class InlineChatController implements IEditorContribution {
       
           static get(editor: ICodeEditor) {
               return editor.getContribution<InlineChatController>(INLINE_CHAT_ID);
           }
       
           private static _decoBlock = ModelDecorationOptions.register({
               description: 'inline-chat',
               showIfCollapsed: false,
               isWholeLine: true,
               className: 'inline-chat-block-selection',
           });
       
           private static _storageKey = 'inline-chat-history';
           private static _promptHistory: string[] = [];
           private _historyOffset: number = -1;
           private _historyUpdate: (prompt: string) => void;
       
           private readonly _store = new DisposableStore();
           private readonly _zone: Lazy<InlineChatZoneWidget>;
           private readonly _ctxHasActiveRequest: IContextKey<boolean>;
           private readonly _ctxLastResponseType: IContextKey<undefined | InlineChatResponseType>;
           private readonly _ctxResponseTypes: IContextKey<undefined | InlineChateResponseTypes>;
           private readonly _ctxDidEdit: IContextKey<boolean>;
           private readonly _ctxUserDidEdit: IContextKey<boolean>;
           private readonly _ctxLastFeedbackKind: IContextKey<'helpful' | 'unhelpful' | ''>;
           private readonly _ctxSupportIssueReporting: IContextKey<boolean>;
       
           private _messages = this._store.add(new Emitter<Message>());
       
           private readonly _onWillStartSession = this._store.add(new Emitter<void>());
           readonly onWillStartSession = this._onWillStartSession.event;
       
           readonly onDidAcceptInput = Event.filter(this._messages.event, m => m === Message.ACCEPT_INPUT, this._store);
           readonly onDidCancelInput = Event.filter(this._messages.event, m => m === Message.CANCEL_INPUT || m === Message.CANCEL_SESSION, this._store);
       
           private readonly _sessionStore: DisposableStore = this._store.add(new DisposableStore());
           private readonly _stashedSession: MutableDisposable<StashedSession> = this._store.add(new MutableDisposable());
           private _activeSession?: Session;
           private _strategy?: EditModeStrategy;
           private _ignoreModelContentChanged = false;
       
           constructor(
               private readonly _editor: ICodeEditor,
               @IInstantiationService private readonly _instaService: IInstantiationService,
               @IInlineChatSessionService private readonly _inlineChatSessionService: IInlineChatSessionService,
               @IEditorWorkerService private readonly _editorWorkerService: IEditorWorkerService,
               @ILogService private readonly _logService: ILogService,
               @IConfigurationService private readonly _configurationService: IConfigurationService,
               @IDialogService private readonly _dialogService: IDialogService,
               @IContextKeyService contextKeyService: IContextKeyService,
               @IAccessibilityService private readonly _accessibilityService: IAccessibilityService,
               @IKeybindingService private readonly _keybindingService: IKeybindingService,
               @ICSChatAccessibilityService private readonly _chatAccessibilityService: ICSChatAccessibilityService,
               @ICSChatAgentService private readonly _chatAgentService: ICSChatAgentService,
               @IBulkEditService private readonly _bulkEditService: IBulkEditService,
               @IStorageService private readonly _storageService: IStorageService,
               @IInlineCSChatVariablesService private readonly chatVariablesService: IInlineCSChatVariablesService,
           ) {
               this._ctxHasActiveRequest = CTX_INLINE_CHAT_HAS_ACTIVE_REQUEST.bindTo(contextKeyService);
               this._ctxDidEdit = CTX_INLINE_CHAT_DID_EDIT.bindTo(contextKeyService);
               this._ctxUserDidEdit = CTX_INLINE_CHAT_USER_DID_EDIT.bindTo(contextKeyService);
               this._ctxResponseTypes = CTX_INLINE_CHAT_RESPONSE_TYPES.bindTo(contextKeyService);
               this._ctxLastResponseType = CTX_INLINE_CHAT_LAST_RESPONSE_TYPE.bindTo(contextKeyService);
               this._ctxLastFeedbackKind = CTX_INLINE_CHAT_LAST_FEEDBACK.bindTo(contextKeyService);
               this._ctxSupportIssueReporting = CTX_INLINE_CHAT_SUPPORT_ISSUE_REPORTING.bindTo(contextKeyService);
               this._zone = new Lazy(() => this._store.add(_instaService.createInstance(InlineChatZoneWidget, this._editor)));
       
               this._store.add(this._editor.onDidChangeModel(async e => {
                   if (this._activeSession || !e.newModelUrl) {
                       return;
                   }
       
                   const existingSession = this._inlineChatSessionService.getSession(this._editor, e.newModelUrl);
                   if (!existingSession) {
                       return;
                   }
       
                   this._log('session RESUMING', e);
                   await this.run({ existingSession });
                   this._log('session done or paused');
               }));
               this._log('NEW controller');
       
               InlineChatController._promptHistory = JSON.parse(_storageService.get(InlineChatController._storageKey, StorageScope.PROFILE, '[]'));
               this._historyUpdate = (prompt: string) => {
                   const idx = InlineChatController._promptHistory.indexOf(prompt);
                   if (idx >= 0) {
                       InlineChatController._promptHistory.splice(idx, 1);
                   }
                   InlineChatController._promptHistory.unshift(prompt);
                   this._historyOffset = -1;
                   this._storageService.store(InlineChatController._storageKey, JSON.stringify(InlineChatController._promptHistory), StorageScope.PROFILE, StorageTarget.USER);
               };
           }
       
           dispose(): void {
               this._strategy?.dispose();
               this._stashedSession.clear();
               if (this._activeSession) {
                   this._inlineChatSessionService.releaseSession(this._activeSession);
               }
               this._store.dispose();
               this._log('controller disposed');
           }
       
           private _log(message: string | Error, ...more: any[]): void {
               if (message instanceof Error) {
                   this._logService.error(message, ...more);
               } else {
                   this._logService.trace(`[IE] (editor:${this._editor.getId()})${message}`, ...more);
               }
           }
       
           getMessage(): string | undefined {
               return this._zone.value.widget.responseContent;
           }
       
           getId(): string {
               return INLINE_CHAT_ID;
           }
       
           private _getMode(): EditMode {
               const editMode = this._configurationService.inspect<EditMode>('inlineChat.mode');
               let editModeValue = editMode.value;
               if (this._accessibilityService.isScreenReaderOptimized() && editModeValue === editMode.defaultValue) {
                   // By default, use preview mode for screen reader users
                   editModeValue = EditMode.Preview;
               }
               return editModeValue!;
           }
       
           getWidget(): InlineChatWidget {
               return this._zone.value.widget;
           }
       
           getWidgetPosition(): Position | undefined {
               return this._zone.value.position;
           }
       
           private _currentRun?: Promise<void>;
       
           async run(options: InlineChatRunOptions | undefined = {}): Promise<void> {
               try {
                   this.finishExistingSession();
                   if (this._currentRun) {
                       await this._currentRun;
                   }
                   this._stashedSession.clear();
                   if (options.initialSelection) {
                       this._editor.setSelection(options.initialSelection);
                   }
                   this._onWillStartSession.fire();
                   this._currentRun = this._nextState(State.CREATE_SESSION, options);
                   await this._currentRun;
       
               } catch (error) {
                   // this should not happen but when it does make sure to tear down the UI and everything
                   onUnexpectedError(error);
                   if (this._activeSession) {
                       this._inlineChatSessionService.releaseSession(this._activeSession);
                   }
                   this[State.PAUSE]();
       
               } finally {
                   this._currentRun = undefined;
               }
           }
       
           joinCurrentRun(): Promise<void> | undefined {
               return this._currentRun;
           }
       
           // ---- state machine
       
           private _showWidget(initialRender: boolean = false, position?: Position) {
               assertType(this._editor.hasModel());
       
               let widgetPosition: Position;
               if (position) {
                   // explicit position wins
                   widgetPosition = position;
               } else if (this._zone.value.position) {
                   // already showing - special case of line 1
                   if (this._zone.value.position.lineNumber === 1) {
                       widgetPosition = this._zone.value.position.delta(-1);
                   } else {
                       widgetPosition = this._zone.value.position;
                   }
               } else {
                   // default to ABOVE the selection
                   widgetPosition = this._editor.getSelection().getStartPosition().delta(-1);
               }
       
               let needsMargin = false;
               if (initialRender) {
                   this._zone.value.setContainerMargins();
               }
       
               if (this._activeSession && (this._activeSession.hasChangedText || this._activeSession.lastExchange)) {
                   widgetPosition = this._activeSession.wholeRange.value.getStartPosition().delta(-1);
               }
               if (this._activeSession) {
                   this._zone.value.updateBackgroundColor(widgetPosition, this._activeSession.wholeRange.value);
               }
               if (this._strategy) {
                   needsMargin = this._strategy.needsMargin();
               }
               if (!this._zone.value.position) {
                   this._zone.value.setWidgetMargins(widgetPosition, !needsMargin ? 0 : undefined);
                   this._zone.value.show(widgetPosition);
               } else {
                   this._zone.value.updatePositionAndHeight(widgetPosition);
               }
           }
       
           protected async _nextState(state: State, options: InlineChatRunOptions): Promise<void> {
               let nextState: State | void = state;
               while (nextState) {
                   this._log('setState to ', nextState);
                   nextState = await this[nextState](options);
               }
           }
       
           private async [State.CREATE_SESSION](options: InlineChatRunOptions): Promise<State.CANCEL | State.INIT_UI | State.PAUSE> {
               assertType(this._activeSession === undefined);
               assertType(this._editor.hasModel());
       
               let session: Session | undefined = options.existingSession;
       
       
               let initPosition: Position | undefined;
               if (options.position) {
                   initPosition = Position.lift(options.position).delta(-1);
                   delete options.position;
               }
       
               this._showWidget(true, initPosition);
       
               this._zone.value.widget.updateInfo(localize('welcome.1', "AI-generated code may be incorrect"));
               this._updatePlaceholder();
       
               if (!session) {
                   const createSessionCts = new CancellationTokenSource();
                   const msgListener = Event.once(this._messages.event)(m => {
                       this._log('state=_createSession) message received', m);
                       if (m === Message.ACCEPT_INPUT) {
                           // user accepted the input before having a session
                           options.autoSend = true;
                           this._zone.value.widget.updateProgress(true);
                           this._zone.value.widget.updateInfo(localize('welcome.2', "Getting ready..."));
                       } else {
                           createSessionCts.cancel();
                       }
                   });
       
                   session = await this._inlineChatSessionService.createSession(
                       this._editor,
                       { editMode: this._getMode(), wholeRange: options.initialRange },
                       createSessionCts.token
                   );
       
                   createSessionCts.dispose();
                   msgListener.dispose();
       
                   if (createSessionCts.token.isCancellationRequested) {
                       return State.PAUSE;
                   }
               }
       
               delete options.initialRange;
               delete options.existingSession;
       
               if (!session) {
                   this._dialogService.info(localize('create.fail', "Failed to start editor chat"), localize('create.fail.detail', "Please consult the error log and try again later."));
                   return State.CANCEL;
               }
       
               switch (session.editMode) {
                   case EditMode.Live:
                       this._strategy = this._instaService.createInstance(LiveStrategy, session, this._editor, this._zone.value.widget);
                       break;
                   case EditMode.Preview:
                       this._strategy = this._instaService.createInstance(PreviewStrategy, session, this._zone.value.widget);
                       break;
                   case EditMode.LivePreview:
                   default:
                       this._strategy = this._instaService.createInstance(LivePreviewStrategy, session, this._editor, this._zone.value.widget);
                       break;
               }
       
               this._activeSession = session;
               return State.INIT_UI;
           }
       
           private async [State.INIT_UI](options: InlineChatRunOptions): Promise<State.WAIT_FOR_INPUT | State.SHOW_RESPONSE | State.APPLY_RESPONSE> {
               assertType(this._activeSession);
       
               // hide/cancel inline completions when invoking IE
               InlineCompletionsController.get(this._editor)?.hide();
       
               this._sessionStore.clear();
       
               const wholeRangeDecoration = this._editor.createDecorationsCollection();
               const updateWholeRangeDecoration = () => {
       
                   const range = this._activeSession!.wholeRange.value;
                   const decorations: IModelDeltaDecoration[] = [];
                   if (!range.isEmpty()) {
                       decorations.push({
                           range,
                           options: InlineChatController._decoBlock
                       });
                   }
                   wholeRangeDecoration.set(decorations);
               };
               this._sessionStore.add(toDisposable(() => wholeRangeDecoration.clear()));
               this._sessionStore.add(this._activeSession.wholeRange.onDidChange(updateWholeRangeDecoration));
               updateWholeRangeDecoration();
       
               this._zone.value.widget.updateSlashCommands(this._activeSession.session.slashCommands ?? []);
               this._updatePlaceholder();
               this._zone.value.widget.updateInfo(this._activeSession.session.message ?? localize('welcome.1', "AI-generated code may be incorrect"));
               this._zone.value.widget.preferredExpansionState = this._activeSession.lastExpansionState;
               this._zone.value.widget.value = this._activeSession.session.input ?? this._activeSession.lastInput?.value ?? this._zone.value.widget.value;
               if (this._activeSession.session.input) {
                   this._zone.value.widget.selectAll();
               }
       
               this._showWidget(true);
       
               this._sessionStore.add(this._editor.onDidChangeModel((e) => {
                   const msg = this._activeSession?.lastExchange
                       ? Message.PAUSE_SESSION // pause when switching models/tabs and when having a previous exchange
                       : Message.CANCEL_SESSION;
                   this._log('model changed, pause or cancel session', msg, e);
                   this._messages.fire(msg);
               }));
       
               const altVersionNow = this._editor.getModel()?.getAlternativeVersionId();
       
               this._sessionStore.add(this._editor.onDidChangeModelContent(e => {
       
                   if (!this._ignoreModelContentChanged && this._strategy?.hasFocus()) {
                       this._ctxUserDidEdit.set(altVersionNow !== this._editor.getModel()?.getAlternativeVersionId());
                   }
       
                   if (this._ignoreModelContentChanged || this._strategy?.hasFocus()) {
                       return;
                   }
       
                   const wholeRange = this._activeSession!.wholeRange;
                   let editIsOutsideOfWholeRange = false;
                   for (const { range } of e.changes) {
                       editIsOutsideOfWholeRange = !Range.areIntersectingOrTouching(range, wholeRange.value);
                   }
       
                   this._activeSession!.recordExternalEditOccurred(editIsOutsideOfWholeRange);
       
                   if (editIsOutsideOfWholeRange) {
                       this._log('text changed outside of whole range, FINISH session');
                       this.finishExistingSession();
                   }
               }));
       
               // Update context key
               this._ctxSupportIssueReporting.set(this._activeSession.provider.supportIssueReporting ?? false);
       
               if (!this._activeSession.lastExchange) {
                   return State.WAIT_FOR_INPUT;
               } else if (options.isUnstashed) {
                   delete options.isUnstashed;
                   return State.APPLY_RESPONSE;
               } else {
                   return State.SHOW_RESPONSE;
               }
           }
       
           private _forcedPlaceholder: string | undefined = undefined;
           setPlaceholder(text: string): void {
               this._forcedPlaceholder = text;
               this._updatePlaceholder();
           }
       
           resetPlaceholder(): void {
               this._forcedPlaceholder = undefined;
               this._updatePlaceholder();
           }
       
           private _updatePlaceholder(): void {
               this._zone.value.widget.placeholder = this._getPlaceholderText();
           }
       
           private _getPlaceholderText(): string {
               let result = this._forcedPlaceholder ?? this._activeSession?.session.placeholder ?? localize('default.placeholder', "Ask a question");
               if (typeof this._forcedPlaceholder === 'undefined' && InlineChatController._promptHistory.length > 0) {
                   const kb1 = this._keybindingService.lookupKeybinding('inlineChat.previousFromHistory')?.getLabel();
                   const kb2 = this._keybindingService.lookupKeybinding('inlineChat.nextFromHistory')?.getLabel();
       
                   if (kb1 && kb2) {
                       result = localize('default.placeholder.history', "{0} ({1}, {2} for history)", result, kb1, kb2);
                   }
               }
               return result;
           }
       
       
           private async [State.WAIT_FOR_INPUT](options: InlineChatRunOptions): Promise<State.ACCEPT | State.CANCEL | State.PAUSE | State.WAIT_FOR_INPUT | State.MAKE_REQUEST> {
               assertType(this._activeSession);
               assertType(this._strategy);
       
               this._updatePlaceholder();
       
               if (options.message) {
                   this.updateInput(options.message);
                   aria.alert(options.message);
                   delete options.message;
               }
       
               let message = Message.NONE;
               if (options.autoSend) {
                   message = Message.ACCEPT_INPUT;
                   delete options.autoSend;
       
               } else {
                   const barrier = new Barrier();
                   const msgListener = Event.once(this._messages.event)(m => {
                       this._log('state=_waitForInput) message received', m);
                       message = m;
                       barrier.open();
                   });
                   await barrier.wait();
                   msgListener.dispose();
               }
       
               this._zone.value.widget.selectAll(false);
       
               if (message & (Message.CANCEL_INPUT | Message.CANCEL_SESSION)) {
                   return State.CANCEL;
               }
       
               if (message & Message.ACCEPT_SESSION) {
                   return State.ACCEPT;
               }
       
               if (message & Message.PAUSE_SESSION) {
                   return State.PAUSE;
               }
       
               if (message & Message.RERUN_INPUT && this._activeSession.lastExchange) {
                   const { lastExchange } = this._activeSession;
                   this._activeSession.addInput(lastExchange.prompt.retry());
                   if (lastExchange.response instanceof ReplyResponse) {
                       try {
                           this._ignoreModelContentChanged = true;
                           await this._strategy.undoChanges(lastExchange.response.modelAltVersionId);
                       } finally {
                           this._ignoreModelContentChanged = false;
                       }
                   }
                   return State.MAKE_REQUEST;
               }
       
               if (!this.getInput()) {
                   return State.WAIT_FOR_INPUT;
               }
       
               const input = this.getInput();
       
               this._historyUpdate(input);
       
               const refer = this._activeSession.session.slashCommands?.some(value => value.refer && input!.startsWith(`/${value.command}`));
               if (refer) {
                   this._log('[IE] seeing refer command, continuing outside editor', this._activeSession.provider.debugName);
                   this._editor.setSelection(this._activeSession.wholeRange.value);
                   let massagedInput = input;
                   if (input.startsWith(chatSubcommandLeader)) {
                       const withoutSubCommandLeader = input.slice(1);
                       const cts = new CancellationTokenSource();
                       this._sessionStore.add(cts);
                       for (const agent of this._chatAgentService.getAgents()) {
                           const commands = await agent.provideSlashCommands(cts.token);
                           if (commands.find((command) => withoutSubCommandLeader.startsWith(command.name))) {
                               massagedInput = `${chatAgentLeader}${agent.id} ${input}`;
                               break;
                           }
                       }
                   }
                   // if agent has a refer command, massage the input to include the agent name
                   this._instaService.invokeFunction(sendRequest, massagedInput);
       
                   if (!this._activeSession.lastExchange) {
                       // DONE when there wasn't any exchange yet. We used the inline chat only as trampoline
                       return State.ACCEPT;
                   }
                   return State.WAIT_FOR_INPUT;
               }
       
               this._activeSession.addInput(new SessionPrompt(input));
               return State.MAKE_REQUEST;
           }
       
           private async [State.MAKE_REQUEST](): Promise<State.APPLY_RESPONSE | State.PAUSE | State.CANCEL | State.ACCEPT> {
               assertType(this._editor.hasModel());
               assertType(this._activeSession);
               assertType(this._strategy);
               assertType(this._activeSession.lastInput);
       
               const inlineCSChatWidget = this._zone.value.widget;
               const slashCommands = inlineCSChatWidget.getSlashCommands();
       
               const requestCts = new CancellationTokenSource();
       
               let message = Message.NONE;
               const msgListener = Event.once(this._messages.event)(m => {
                   this._log('state=_makeRequest) message received', m);
                   message = m;
                   requestCts.cancel();
               });
       
               const typeListener = this._zone.value.widget.onDidChangeInput(() => requestCts.cancel());
       
               const requestClock = StopWatch.create();
               const request: IInlineCSChatRequest = {
                   requestId: generateUuid(),
                   prompt: this._activeSession.lastInput.value,
                   attempt: this._activeSession.lastInput.attempt,
                   selection: this._editor.getSelection(),
                   wholeRange: this._activeSession.wholeRange.value,
                   live: this._activeSession.editMode !== EditMode.Preview, // TODO@jrieken let extension know what document is used for previewing
                   variables: {}
               };
       
               const parsedRequest = await this._instaService.createInstance(InlineCSChatRequestParser).parseChatRequest('', this._activeSession.lastInput.value, slashCommands);
               if ('parts' in parsedRequest) {
                   const varResult = await this.chatVariablesService.resolveVariables(parsedRequest, requestCts.token);
                   request.variables = varResult.variables;
                   request.prompt = varResult.prompt;
               }
       
               const modelAltVersionIdNow = this._activeSession.textModelN.getAlternativeVersionId();
               const progressEdits: TextEdit[][] = [];
       
               const progressiveEditsAvgDuration = new MovingAverage();
               const progressiveEditsCts = new CancellationTokenSource(requestCts.token);
               const progressiveEditsClock = StopWatch.create();
               const progressiveEditsQueue = new Queue();
       
               let progressiveChatResponse: IInlineChatMessageAppender | undefined;
       
               const progress = new Progress<IInlineCSChatProgressItem>(data => {
                   this._log('received chunk', data, request);
       
                   if (requestCts.token.isCancellationRequested) {
                       return;
                   }
       
                   if (data.message) {
                       this._zone.value.widget.updateToolbar(false);
                       this._zone.value.widget.updateInfo(data.message);
                   }
                   if (data.slashCommand) {
                       const valueNow = this.getInput();
                       if (!valueNow.startsWith('/')) {
                           this._zone.value.widget.updateSlashCommandUsed(data.slashCommand);
                       }
                   }
                   if (data.edits?.length) {
                       if (!request.live) {
                           throw new Error('Progress in NOT supported in non-live mode');
                       }
                       progressEdits.push(data.edits);
                       progressiveEditsAvgDuration.update(progressiveEditsClock.elapsed());
                       progressiveEditsClock.reset();
       
                       progressiveEditsQueue.queue(async () => {
       
                           const startThen = this._activeSession!.wholeRange.value.getStartPosition();
       
                           // making changes goes into a queue because otherwise the async-progress time will
                           // influence the time it takes to receive the changes and progressive typing will
                           // become infinitely fast
                           await this._makeChanges(data.edits!, data.editsShouldBeInstant
                               ? undefined
                               : { duration: progressiveEditsAvgDuration.value, token: progressiveEditsCts.token }
                           );
       
                           // reshow the widget if the start position changed or shows at the wrong position
                           const startNow = this._activeSession!.wholeRange.value.getStartPosition();
                           if (!startNow.equals(startThen) || !this._zone.value.position?.equals(startNow)) {
                               this._showWidget(false, startNow.delta(-1));
                           }
                       });
                   }
                   if (data.markdownFragment) {
                       if (!progressiveChatResponse) {
                           const message = {
                               message: new MarkdownString(data.markdownFragment, { supportThemeIcons: true, supportHtml: true, isTrusted: false }),
                               providerId: this._activeSession!.provider.debugName,
                               requestId: request.requestId,
                           };
                           progressiveChatResponse = this._zone.value.widget.updateChatMessage(message, true);
                       } else {
                           progressiveChatResponse.appendContent(data.markdownFragment);
                       }
                   }
               });
       
               let a11yResponse: string | undefined;
               const a11yVerboseInlineChat = this._configurationService.getValue<boolean>('accessibility.verbosity.inlineChat') === true;
               const requestId = this._chatAccessibilityService.acceptRequest();
       
               const task = this._activeSession.provider.provideResponse(this._activeSession.session, request, progress, requestCts.token);
               this._log('request started', this._activeSession.provider.debugName, this._activeSession.session, request);
       
               let response: ReplyResponse | ErrorResponse | EmptyResponse;
               let reply: IInlineCSChatResponse | null | undefined;
               try {
                   this._zone.value.widget.updateChatMessage(undefined);
                   this._zone.value.widget.updateMarkdownMessage(undefined);
                   this._zone.value.widget.updateFollowUps(undefined);
                   this._zone.value.widget.updateProgress(true);
                   this._zone.value.widget.updateInfo(!this._activeSession.lastExchange ? localize('thinking', "Thinking\u2026") : '');
                   this._ctxHasActiveRequest.set(true);
                   reply = await raceCancellationError(Promise.resolve(task), requestCts.token);
       
                   if (progressiveEditsQueue.size > 0) {
                       // we must wait for all edits that came in via progress to complete
                       await Event.toPromise(progressiveEditsQueue.onDrained);
                   }
                   if (progressiveChatResponse) {
                       progressiveChatResponse.cancel();
                   }
       
                   if (!reply) {
                       response = new EmptyResponse();
                       a11yResponse = localize('empty', "No results, please refine your input and try again");
                   } else {
                       const markdownContents = reply.message ?? new MarkdownString('', { supportThemeIcons: true, supportHtml: true, isTrusted: false });
                       const replyResponse = response = this._instaService.createInstance(ReplyResponse, reply, markdownContents, this._activeSession.textModelN.uri, modelAltVersionIdNow, progressEdits, request.requestId);
       
                       for (let i = progressEdits.length; i < replyResponse.allLocalEdits.length; i++) {
                           await this._makeChanges(replyResponse.allLocalEdits[i], undefined);
                       }
       
                       const a11yMessageResponse = renderMarkdownAsPlaintext(replyResponse.mdContent);
       
                       a11yResponse = a11yVerboseInlineChat
                           ? a11yMessageResponse ? localize('editResponseMessage2', "{0}, also review proposed changes in the diff editor.", a11yMessageResponse) : localize('editResponseMessage', "Review proposed changes in the diff editor.")
                           : a11yMessageResponse;
                   }
       
               } catch (e) {
                   response = new ErrorResponse(e);
                   a11yResponse = (<ErrorResponse>response).message;
       
               } finally {
                   this._ctxHasActiveRequest.set(false);
                   this._zone.value.widget.updateProgress(false);
                   this._zone.value.widget.updateInfo('');
                   this._zone.value.widget.updateToolbar(true);
                   this._log('request took', requestClock.elapsed(), this._activeSession.provider.debugName);
                   this._chatAccessibilityService.acceptResponse(a11yResponse, requestId);
               }
       
               progressiveEditsCts.dispose(true);
               requestCts.dispose();
               msgListener.dispose();
               typeListener.dispose();
       
               if (request.live && !(response instanceof ReplyResponse)) {
                   this._strategy?.undoChanges(modelAltVersionIdNow);
               }
       
               this._activeSession.addExchange(new SessionExchange(this._activeSession.lastInput, response));
       
               if (message & Message.CANCEL_SESSION) {
                   return State.CANCEL;
               } else if (message & Message.PAUSE_SESSION) {
                   return State.PAUSE;
               } else if (message & Message.ACCEPT_SESSION) {
                   return State.ACCEPT;
               } else {
                   return State.APPLY_RESPONSE;
               }
           }
       
           private async[State.APPLY_RESPONSE](): Promise<State.SHOW_RESPONSE | State.CANCEL> {
               assertType(this._activeSession);
               assertType(this._strategy);
       
               const { response } = this._activeSession.lastExchange!;
               if (response instanceof ReplyResponse && response.workspaceEdit) {
                   // this reply cannot be applied in the normal inline chat UI and needs to be handled off to workspace edit
                   this._bulkEditService.apply(response.workspaceEdit, { showPreview: true });
                   return State.CANCEL;
               }
               return State.SHOW_RESPONSE;
           }
       
           private async _makeChanges(edits: TextEdit[], opts: ProgressingEditsOptions | undefined) {
               assertType(this._activeSession);
               assertType(this._strategy);
       
               const moreMinimalEdits = await this._editorWorkerService.computeMoreMinimalEdits(this._activeSession.textModelN.uri, edits);
               this._log('edits from PROVIDER and after making them MORE MINIMAL', this._activeSession.provider.debugName, edits, moreMinimalEdits);
       
               if (moreMinimalEdits?.length === 0) {
                   // nothing left to do
                   return;
               }
       
               const actualEdits = !opts && moreMinimalEdits ? moreMinimalEdits : edits;
               const editOperations = actualEdits.map(TextEdit.asEditOperation);
       
               try {
                   this._ignoreModelContentChanged = true;
                   this._activeSession.wholeRange.trackEdits(editOperations);
                   if (opts) {
                       await this._strategy.makeProgressiveChanges(editOperations, opts);
                   } else {
                       await this._strategy.makeChanges(editOperations);
                   }
                   this._ctxDidEdit.set(this._activeSession.hasChangedText);
               } finally {
                   this._ignoreModelContentChanged = false;
               }
           }
       
           private async[State.SHOW_RESPONSE](): Promise<State.WAIT_FOR_INPUT | State.CANCEL> {
               assertType(this._activeSession);
               assertType(this._strategy);
       
               const { response } = this._activeSession.lastExchange!;
       
               this._ctxLastResponseType.set(response instanceof ReplyResponse ? response.raw.type : undefined);
       
               let responseTypes: InlineChateResponseTypes | undefined;
               for (const { response } of this._activeSession.exchanges) {
       
                   const thisType = response instanceof ReplyResponse
                       ? response.responseType
                       : undefined;
       
                   if (responseTypes === undefined) {
                       responseTypes = thisType;
                   } else if (responseTypes !== thisType) {
                       responseTypes = InlineChateResponseTypes.Mixed;
                       break;
                   }
               }
               this._ctxResponseTypes.set(responseTypes);
               this._ctxDidEdit.set(this._activeSession.hasChangedText);
       
               if (response instanceof EmptyResponse) {
                   // show status message
                   const status = localize('empty', "No results, please refine your input and try again");
                   this._zone.value.widget.updateStatus(status, { classes: ['warn'] });
                   return State.WAIT_FOR_INPUT;
       
               } else if (response instanceof ErrorResponse) {
                   // show error
                   if (!response.isCancellation) {
                       this._zone.value.widget.updateStatus(response.message, { classes: ['error'] });
                   }
       
               } else if (response instanceof ReplyResponse) {
                   // real response -> complex...
                   this._zone.value.widget.updateStatus('');
                   const message = { message: response.mdContent, providerId: this._activeSession.provider.debugName, requestId: response.requestId };
                   this._zone.value.widget.updateChatMessage(message);
       
                   //this._zone.value.widget.updateMarkdownMessage(response.mdContent);
                   this._activeSession.lastExpansionState = this._zone.value.widget.expansionState;
                   this._zone.value.widget.updateToolbar(true);
       
                   await this._strategy.renderChanges(response);
       
                   if (this._activeSession.provider.provideFollowups) {
                       const followupCts = new CancellationTokenSource();
                       const msgListener = Event.once(this._messages.event)(() => {
                           followupCts.cancel();
                       });
                       const followupTask = this._activeSession.provider.provideFollowups(this._activeSession.session, response.raw, followupCts.token);
                       this._log('followup request started', this._activeSession.provider.debugName, this._activeSession.session, response.raw);
                       raceCancellation(Promise.resolve(followupTask), followupCts.token).then(followupReply => {
                           if (followupReply && this._activeSession) {
                               this._log('followup request received', this._activeSession.provider.debugName, this._activeSession.session, followupReply);
                               this._zone.value.widget.updateFollowUps(followupReply, followup => {
                                   this.updateInput(followup.message);
                                   this.acceptInput();
                               });
                           }
                       }).finally(() => {
                           msgListener.dispose();
                           followupCts.dispose();
                       });
                   }
               }
               this._showWidget(false);
       
               return State.WAIT_FOR_INPUT;
           }
       
           private async[State.PAUSE]() {
       
               this._ctxDidEdit.reset();
               this._ctxUserDidEdit.reset();
               this._ctxLastResponseType.reset();
               this._ctxLastFeedbackKind.reset();
               this._ctxSupportIssueReporting.reset();
       
               this._zone.value.hide();
       
               // Return focus to the editor only if the current focus is within the editor widget
               if (this._editor.hasWidgetFocus()) {
                   this._editor.focus();
               }
       
       
               this._strategy?.dispose();
               this._strategy = undefined;
               this._activeSession = undefined;
           }
       
           private async[State.ACCEPT]() {
               assertType(this._activeSession);
               assertType(this._strategy);
               this._sessionStore.clear();
       
               try {
                   await this._strategy.apply();
               } catch (err) {
                   this._dialogService.error(localize('err.apply', "Failed to apply changes.", toErrorMessage(err)));
                   this._log('FAILED to apply changes');
                   this._log(err);
               }
       
               this._inlineChatSessionService.releaseSession(this._activeSession);
       
               this[State.PAUSE]();
           }
       
           private async[State.CANCEL]() {
               assertType(this._activeSession);
               assertType(this._strategy);
               this._sessionStore.clear();
       
               const mySession = this._activeSession;
       
               try {
                   await this._strategy.cancel();
               } catch (err) {
                   this._dialogService.error(localize('err.discard', "Failed to discard changes.", toErrorMessage(err)));
                   this._log('FAILED to discard changes');
                   this._log(err);
               }
       
               this[State.PAUSE]();
       
               this._stashedSession.clear();
               if (!mySession.isUnstashed && mySession.lastExchange) {
                   // only stash sessions that had edits
                   this._stashedSession.value = this._instaService.createInstance(StashedSession, this._editor, mySession);
               } else {
                   this._inlineChatSessionService.releaseSession(mySession);
               }
           }
       
           // ---- controller API
       
           acceptInput(): void {
               this._messages.fire(Message.ACCEPT_INPUT);
           }
       
           updateInput(text: string, selectAll = true): void {
               this._zone.value.widget.value = text;
               if (selectAll) {
                   this._zone.value.widget.selectAll();
               }
           }
       
           getInput(): string {
               return this._zone.value.widget.value;
           }
       
           regenerate(): void {
               this._messages.fire(Message.RERUN_INPUT);
           }
       
           cancelCurrentRequest(): void {
               this._messages.fire(Message.CANCEL_INPUT | Message.CANCEL_REQUEST);
           }
       
           arrowOut(up: boolean): void {
               if (this._zone.value.position && this._editor.hasModel()) {
                   const { column } = this._editor.getPosition();
                   const { lineNumber } = this._zone.value.position;
                   const newLine = up ? lineNumber : lineNumber + 1;
                   this._editor.setPosition({ lineNumber: newLine, column });
                   this._editor.focus();
               }
           }
       
           focus(): void {
               this._zone.value.widget.focus();
           }
       
           hasFocus(): boolean {
               return this._zone.value.widget.hasFocus();
           }
       
           populateHistory(up: boolean) {
               const len = InlineChatController._promptHistory.length;
               if (len === 0) {
                   return;
               }
               const pos = (len + this._historyOffset + (up ? 1 : -1)) % len;
               const entry = InlineChatController._promptHistory[pos];
       
               this._zone.value.widget.value = entry;
               this._zone.value.widget.selectAll();
               this._historyOffset = pos;
           }
       
           viewInChat() {
               if (this._activeSession?.lastExchange?.response instanceof ReplyResponse) {
                   this._instaService.invokeFunction(showMessageResponse, this._activeSession.lastExchange.prompt.value, this._activeSession.lastExchange.response.mdContent.value);
               }
           }
       
           updateExpansionState(expand: boolean) {
               if (this._activeSession) {
                   const expansionState = expand ? ExpansionState.EXPANDED : ExpansionState.CROPPED;
                   this._zone.value.widget.updateMarkdownMessageExpansionState(expansionState);
                   this._activeSession.lastExpansionState = expansionState;
               }
           }
       
           feedbackLast(kind: InlineCSChatResponseFeedbackKind) {
               if (this._activeSession?.lastExchange && this._activeSession.lastExchange.response instanceof ReplyResponse) {
                   this._activeSession.provider.handleInlineChatResponseFeedback?.(this._activeSession.session, this._activeSession.lastExchange.response.raw, kind);
                   switch (kind) {
                       case InlineCSChatResponseFeedbackKind.Helpful:
                           this._ctxLastFeedbackKind.set('helpful');
                           break;
                       case InlineCSChatResponseFeedbackKind.Unhelpful:
                           this._ctxLastFeedbackKind.set('unhelpful');
                           break;
                       default:
                           break;
                   }
                   this._zone.value.widget.updateStatus('Thank you for your feedback!', { resetAfter: 1250 });
               }
           }
       
           createSnapshot(): void {
               if (this._activeSession && !this._activeSession.textModel0.equalsTextBuffer(this._activeSession.textModelN.getTextBuffer())) {
                   this._activeSession.createSnapshot();
               }
           }
       
           acceptSession(): void {
               if (this._activeSession?.lastExchange && this._activeSession.lastExchange.response instanceof ReplyResponse) {
                   this._activeSession.provider.handleInlineChatResponseFeedback?.(this._activeSession.session, this._activeSession.lastExchange.response.raw, InlineCSChatResponseFeedbackKind.Accepted);
               }
               this._messages.fire(Message.ACCEPT_SESSION);
           }
       
           async cancelSession() {
       
               let result: string | undefined;
               if (this._activeSession) {
       
                   const diff = await this._editorWorkerService.computeDiff(this._activeSession.textModel0.uri, this._activeSession.textModelN.uri, { ignoreTrimWhitespace: false, maxComputationTimeMs: 5000, computeMoves: false }, 'advanced');
                   result = this._activeSession.asChangedText(diff?.changes ?? []);
       
                   if (this._activeSession.lastExchange && this._activeSession.lastExchange.response instanceof ReplyResponse) {
                       this._activeSession.provider.handleInlineChatResponseFeedback?.(this._activeSession.session, this._activeSession.lastExchange.response.raw, InlineCSChatResponseFeedbackKind.Undone);
                   }
               }
       
               this._messages.fire(Message.CANCEL_SESSION);
               return result;
           }
       
           finishExistingSession(): void {
               if (this._activeSession) {
                   if (this._activeSession.editMode === EditMode.Preview) {
                       this._log('finishing existing session, using CANCEL', this._activeSession.editMode);
                       this.cancelSession();
                   } else {
                       this._log('finishing existing session, using APPLY', this._activeSession.editMode);
                       this.acceptSession();
                   }
               }
           }
       
           unstashLastSession(): Session | undefined {
               return this._stashedSession.value?.unstash();
           }
       }
       
       
       class StashedSession {
       
           private readonly _listener: IDisposable;
           private readonly _ctxHasStashedSession: IContextKey<boolean>;
           private _session: Session | undefined;
       
           constructor(
               editor: ICodeEditor,
               session: Session,
               @IContextKeyService contextKeyService: IContextKeyService,
               @IInlineChatSessionService private readonly _sessionService: IInlineChatSessionService,
               @ILogService private readonly _logService: ILogService,
           ) {
               this._ctxHasStashedSession = CTX_INLINE_CHAT_HAS_STASHED_SESSION.bindTo(contextKeyService);
       
               // keep session for a little bit, only release when user continues to work (type, move cursor, etc.)
               this._session = session;
               this._ctxHasStashedSession.set(true);
               this._listener = Event.once(Event.any(editor.onDidChangeCursorSelection, editor.onDidChangeModelContent, editor.onDidChangeModel))(() => {
                   this._session = undefined;
                   this._sessionService.releaseSession(session);
                   this._ctxHasStashedSession.reset();
               });
           }
       
           dispose() {
               this._listener.dispose();
               this._ctxHasStashedSession.reset();
               if (this._session) {
                   this._sessionService.releaseSession(this._session);
               }
           }
       
           unstash(): Session | undefined {
               if (!this._session) {
                   return undefined;
               }
               this._listener.dispose();
               const result = this._session;
               result.markUnstashed();
               this._session = undefined;
               this._logService.debug('[IE] Unstashed session');
               return result;
           }
       
       }
       
       async function showMessageResponse(accessor: ServicesAccessor, query: string, response: string) {
           const chatService = accessor.get(ICSChatService);
           const providerId = chatService.getProviderInfos()[0]?.id;
       
           const chatWidgetService = accessor.get(ICSChatWidgetService);
           const widget = await chatWidgetService.revealViewForProvider(providerId);
           if (widget && widget.viewModel) {
               chatService.addCompleteRequest(widget.viewModel.sessionId, query, { message: response });
               widget.focusLastMessage();
           }
       }
       
       async function sendRequest(accessor: ServicesAccessor, query: string) {
           const chatService = accessor.get(ICSChatService);
           const widgetService = accessor.get(ICSChatWidgetService);
       
           const providerId = chatService.getProviderInfos()[0]?.id;
           const widget = await widgetService.revealViewForProvider(providerId);
           if (!widget) {
               return;
           }
       
           widget.acceptInput(query);
       }
       
        "#;
        let language = "typescript";
        let tree_sitter_parsing = TSLanguageParsing::init();
        let ts_language_config = tree_sitter_parsing
            .for_lang(language)
            .expect("test to work");
        ts_language_config.capture_class_data(source_code.as_bytes());
        let outline = ts_language_config.generate_file_outline_str(source_code.as_bytes());
        dbg!(&outline);
        assert!(false);
    }
}
