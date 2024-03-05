use crate::chunking::languages::TSLanguageConfig;

pub fn javascript_language_config() -> TSLanguageConfig {
    TSLanguageConfig {
        language_ids: &["Javascript", "JSX", "javascript", "jsx"],
        file_extensions: &["js", "jsx"],
        grammar: tree_sitter_javascript::language,
        namespaces: vec![vec![
            //variables
            "constant",
            "variable",
            "property",
            "function",
            "method",
            "generator",
            // types
            "class",
            // misc.
            "label",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect()],
        documentation_query: vec!["((comment) @comment
        (#match? @comment \"^\\\\/\\\\*\\\\*\")) @docComment"
            .to_owned()],
        function_query: vec!["[
				(function
					name: (identifier)? @identifier
                    parameters: (formal_parameters)? @parameters
					body: (statement_block
                    (lexical_declaration
                      (variable_declarator
                        name: (identifier) @variable.name
                        type: (type_annotation)? @variable.type
                      )
                    )*
                  )? @body)
				(function_declaration
					name: (identifier)? @identifier
                    parameters: (formal_parameters)? @parameters
					body: (statement_block
                    (lexical_declaration
                      (variable_declarator
                        name: (identifier) @variable.name
                        type: (type_annotation)? @variable.type
                      )
                    )*
                  )? @body)
				(generator_function
					name: (identifier)? @identifier
                    parameters: (formal_parameters)? @parameters
					body: (statement_block
                    (lexical_declaration
                      (variable_declarator
                        name: (identifier) @variable.name
                        type: (type_annotation)? @variable.type
                      )
                    )*
                  )? @body)
				(generator_function_declaration
					name: (identifier)? @identifier
                    parameters: (formal_parameters)? @parameters
					body: (statement_block
                    (lexical_declaration
                      (variable_declarator
                        name: (identifier) @variable.name
                        type: (type_annotation)? @variable.type
                      )
                    )*
                  )? @body)
				(method_definition
					name: (property_identifier)? @identifier
                    parameters: (formal_parameters)? @parameters
					body: (statement_block
                    (lexical_declaration
                      (variable_declarator
                        name: (identifier) @variable.name
                        type: (type_annotation)? @variable.type
                      )
                    )*
                  )? @body)
				(arrow_function
                    parameters: (formal_parameters)? @parameters
					body: (statement_block
                    (lexical_declaration
                      (variable_declarator
                        name: (identifier) @variable.name
                        type: (type_annotation)? @variable.type
                      )
                    )*
                  )? @body)
			] @function"
            .to_owned()],
        construct_types: vec![
            "program",
            "class_declaration",
            "function_declaration",
            "function",
            "method_definition",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect(),
        expression_statements: vec![
            "call_expression",
            "expression_statement",
            "variable_declaration",
            "public_field_definition",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect(),
        class_query: vec![
            "[(class_declaration name: (identifier)? @identifier)] @class_declaration".to_owned(),
        ],
        r#type_query: vec![],
        namespace_types: vec![],
        hoverable_query: r#"
        [(identifier)
         (property_identifier)
         (shorthand_property_identifier)
         (shorthand_property_identifier_pattern)
         (private_property_identifier)
         (statement_identifier)] @hoverable
        "#
        .to_owned(),
        comment_prefix: "//".to_owned(),
        end_of_line: Some(";".to_owned()),
        import_statement: vec!["[(import_statement)] @import_type".to_owned()],
        block_start: Some("{".to_owned()),
        vairable_indentifier_queries: vec![
            "((lexical_declaration (variable_declarator (identifier) @identifier)))".to_owned(),
        ],
    }
}
