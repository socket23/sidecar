/// We want to parse the rust language here and provide the language config
/// for it
use crate::chunking::languages::TSLanguageConfig;

pub fn rust_language_config() -> TSLanguageConfig {
    TSLanguageConfig {
        language_ids: &["Rust", "rust"],
        file_extensions: &["rs"],
        grammar: tree_sitter_rust::language,
        namespaces: vec![vec![
            // variables
            "const",
            "function",
            "variable",
            // types
            "struct",
            "enum",
            "union",
            "typedef",
            "interface",
            // fields
            "field",
            "enumerator",
            // namespacing
            "module",
            // misc
            "label",
            "lifetime",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect()],
        documentation_query: vec![
            "((line_comment) @comment
            (#match? @comment \"^///\")) @docComment"
                .to_owned(),
            "((line_comment) @comment
                (#match? @comment \"^//!\")) @moduleDocComment"
                .to_owned(),
        ],
        function_query: vec!["[(function_item
        	name: (identifier)? @identifier
            parameters: (parameters)? @parameters
            return_type: (generic_type)? @return_type
            body: (block) @body)
        ] @function"
            .to_owned()],
        construct_types: vec![
            "source_file",   // Represents the entire Rust source file.
            "struct_item",   // Represents the declaration of a struct.
            "enum_item",     // Represents the declaration of an enum.
            "trait_item",    // Represents the declaration of a trait.
            "impl_item",     // Represents an implementation block.
            "function_item", // Represents a standalone function declaration.
            // "method_item",   // Represents a method within an impl block.
            // "use_item", // Represents the use keyword to import modules or paths.
            "mod_item", // Represents a module declaration.
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect(),
        expression_statements: vec!["let_declaration", "expression_statement", "call_expression"]
            .into_iter()
            .map(|s| s.to_owned())
            .collect(),
        class_query: vec!["[
                (struct_item name: (type_identifier)? @identifier)
                (impl_item type: (type_identifier)? @identifier)
            ] @class_declaration"
            .to_owned()],
        r#type_query: vec![],
        namespace_types: vec![],
        hoverable_query: r#"
        [(identifier)
         (shorthand_field_identifier)
         (field_identifier)
         (type_identifier)] @hoverable
        "#
        .to_owned(),
        comment_prefix: "//".to_owned(),
        end_of_line: Some(";".to_owned()),
        import_statement: vec!["[(use_declaration)] @import_type".to_owned()],
        block_start: Some("{".to_owned()),
    }
}
