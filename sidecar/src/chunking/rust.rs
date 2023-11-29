/// We want to parse the rust language here and provide the language config
/// for it
use crate::chunking::languages::TSLanguageConfig;

pub fn rust_language_config() -> TSLanguageConfig {
    TSLanguageConfig {
        language_ids: &["Rust", "rust"],
        file_extensions: &["rs"],
        grammar: tree_sitter_rust::language,
        namespaces: vec![
            "const",
            "var",
            "func",
            "module",
            "struct",
            "interface",
            "type",
            "member",
            "label",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect(),
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
            "method_item",   // Represents a method within an impl block.
            "mod_item",      // Represents a module declaration.
            "use_item",      // Represents the use keyword to import modules or paths.
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
    }
}
