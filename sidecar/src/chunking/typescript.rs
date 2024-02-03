use crate::chunking::languages::TSLanguageConfig;

pub fn typescript_language_config() -> TSLanguageConfig {
    TSLanguageConfig {
        language_ids: &["Typescript", "TSX", "typescript", "tsx"],
        file_extensions: &["ts", "tsx", "jsx", "mjs"],
        grammar: tree_sitter_typescript::language_tsx,
        namespaces: vec![vec![
            "constant",
            "variable",
            "property",
            "parameter",
            // functions
            "function",
            "method",
            "generator",
            // types
            "alias",
            "enum",
            "enumerator",
            "class",
            "interface",
            // misc.
            "label",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect()],
        documentation_query: vec!["((comment) @comment
        (#match? @comment \"^\\\\/\\\\*\\\\*\")) @docComment"
            .to_owned(), "(comment) @comment".to_owned()],
        function_query: vec!["[
            (function
                name: (identifier)? @identifier
                parameters: (formal_parameters)? @parameters
                return_type: (type_annotation)? @return_type
                body: (statement_block) @body)
            (function_declaration
                name: (identifier)? @identifier
                parameters: (formal_parameters)? @parameters
                return_type: (type_annotation)? @return_type
                body: (statement_block) @body)
            (generator_function
                name: (identifier)? @identifier
                parameters: (formal_parameters)? @parameters
                return_type: (type_annotation)? @return_type
                body: (statement_block) @body)
            (generator_function_declaration
                name: (identifier)? @identifier
                parameters: (formal_parameters)? @parameters
                return_type: (type_annotation)? @return_type
                body: (statement_block) @body)
            (method_definition
                name: (property_identifier)? @identifier
                parameters: (formal_parameters)? @parameters
                return_type: (type_annotation)? @return_type
                body: (statement_block) @body)
            (arrow_function
                body: (statement_block) @body
                parameters: (formal_parameters)? @parameters
                return_type: (type_annotation)? @return_type)
            ] @function"
            .to_owned()],
        construct_types: vec![
            "program",
            "interface_declaration",
            "class_declaration",
            "function_declaration",
            "function",
            "type_alias_declaration",
            "method_definition",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect(),
        expression_statements: vec![
            "lexical_declaration",
            "expression_statement",
            "public_field_definition",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect(),
        class_query: vec![
            "[(abstract_class_declaration name: (type_identifier)? @identifier) (class_declaration name: (type_identifier)? @identifier)] @class_declaration"
                .to_owned(),
        ],
        r#type_query: vec![
            "[(type_alias_declaration name: (type_identifier) @identifier)] @type_declaration"
                .to_owned(),
        ],
        namespace_types: vec![
            "export_statement".to_owned(),
        ],
        hoverable_query: r#"
        [(identifier)
         (property_identifier)
         (shorthand_property_identifier)
         (shorthand_property_identifier_pattern)
         (statement_identifier)
         (type_identifier)] @hoverable
        "#.to_owned(),
        scope_query: r#"
        [
            (statement_block)
            (class_body)
            (arrow_function)
            (object)
            (function !name)
            (function_declaration)
            (generator_function_declaration)
            (for_statement)
            (for_in_statement)
            (switch_case)
            (catch_clause)
            (sequence_expression)
            (property_signature)
          ] @local.scope
          
          (function_declaration
            (identifier) @hoist.definition.function)
          
          (generator_function_declaration
            (identifier) @hoist.definition.generator)
          
          (formal_parameters
            (required_parameter
              (identifier) @local.definition.parameter))
          (formal_parameters
            (optional_parameter
              (identifier) @local.definition.parameter))
          
          
          (rest_pattern
            (identifier) @local.definition.variable)
          
          (assignment_pattern
            (identifier) @local.definition.variable
            (identifier) @local.reference)
          
          (array_pattern
            (identifier) @local.definition.variable)
          
          (object_pattern
            (shorthand_property_identifier_pattern) @local.definition.variable)
          
          (variable_declaration
            (variable_declarator . (identifier) @local.definition.variable))
          
          (lexical_declaration
            "const"
            (variable_declarator . (identifier) @local.definition.constant))
          
          (lexical_declaration
            "let"
            (variable_declarator . (identifier) @local.definition.variable))
          
          (assignment_expression
            .
            (identifier) @local.definition.variable)
          
          (method_definition
            (property_identifier) @local.definition.method)
          
          (class_declaration
            (type_identifier) @local.definition.class)
          
          (arrow_function
            (identifier) @local.definition.variable)
          
          
          
          (import_statement
            (import_clause (identifier) @local.import))
          
          (import_statement
            (import_clause
              (named_imports
                [(import_specifier !alias (identifier) @local.import)
                 (import_specifier alias: (identifier) @local.import)])))
          
          (for_in_statement 
            left: (identifier) @local.definition.variable)
          
          (labeled_statement
            (statement_identifier) @local.definition.label)
          
          (type_alias_declaration
            name:
            (type_identifier) @local.definition.alias)
          
          (type_parameters
            (type_parameter
              (type_identifier) @local.definition))
          
          (enum_declaration
            (identifier) @local.definition.enum)
          
          (enum_body
            (property_identifier) @local.definition.enumerator)
          (enum_body
            (enum_assignment
              (property_identifier) @local.definition.enumerator))
          
          (abstract_class_declaration
            (type_identifier) @local.definition.class)
          
          (public_field_definition
            (property_identifier) @local.definition.property)
          
          (abstract_method_signature
            (property_identifier) @local.definition.property)
          
          (interface_declaration
            (type_identifier) @local.definition.interface)
          
          (catch_clause
            (identifier) @local.definition.variable)
          
          
          
          (expression_statement (identifier) @local.reference)
          
          (object
            (pair
              (identifier) @local.reference))
          
          (object
            (shorthand_property_identifier) @local.reference)
          
          
          (array
            (identifier) @local.reference)
          
          (new_expression
            (identifier) @local.reference)
          
          (return_statement 
            (identifier) @local.reference)
          
          (yield_expression
            (identifier) @local.reference)
          
          (call_expression
            (identifier) @local.reference)
          
          (arguments
            (identifier) @local.reference)
          
          (type_arguments
            (type_identifier) @local.reference)
          
          (subscript_expression
            (identifier) @local.reference)
          
          (member_expression
            (identifier) @local.reference)
          
          (nested_identifier
            .
            (identifier) @local.reference)
          
          (await_expression 
            (identifier) @local.reference)
          
          (binary_expression
            (identifier) @local.reference)
          
          (unary_expression
            (identifier) @local.reference)
          
          (update_expression
            (identifier) @local.reference)
          
          (augmented_assignment_expression
            (identifier) @local.reference)
          
          (parenthesized_expression
            (identifier) @local.reference)
          
          (sequence_expression
            (identifier) @local.reference)
          
          (ternary_expression
            (identifier) @local.reference)
          
          (spread_element
            (identifier) @local.reference)
          
          (export_statement
            (export_clause
              (export_specifier name: (identifier) @local.reference)))
          
          (export_statement
            (identifier) @local.reference)
          
          (for_in_statement 
            right: (identifier) @local.reference)
          
          (break_statement (statement_identifier) @local.reference)
          
          (continue_statement (statement_identifier) @local.reference)
          
          
          
          
          (type_alias_declaration
            value:
            (type_identifier) @local.reference)
          
          (parenthesized_type
            (type_identifier) @local.reference)
          
          (array_type
            (type_identifier) @local.reference)
          
          (conditional_type
            (type_identifier) @local.reference)
          
          (flow_maybe_type
            (type_identifier) @local.reference)
          
          (generic_type
            (type_identifier) @local.reference)
          
          (intersection_type
            (type_identifier) @local.reference)
          
          (union_type
            (type_identifier) @local.reference)
          
          (function_type
            (type_identifier) @local.reference)
          
          (index_type_query
            (type_identifier) @local.reference)
          
          (as_expression
            (identifier) @local.reference
            (type_identifier) @local.reference)
          
          (type_annotation
            (type_identifier) @local.reference)
          
          (tuple_type
            (type_identifier) @local.reference)
          
          (lookup_type
            (type_identifier) @local.reference)
          
          (nested_type_identifier
            .
            (identifier) @local.reference)
          
          (type_predicate_annotation
            (type_predicate
              (identifier) @local.reference
              (type_identifier) @local.reference))
          
          (jsx_expression
            (identifier) @local.reference)
          
          (jsx_opening_element
            (identifier) @local.reference)
          
          (jsx_closing_element
            (identifier) @local.reference)
          
          (jsx_self_closing_element
            (identifier) @local.reference)
          
        "#.to_owned(),
        comment_prefix: "//".to_owned(),
    }
}
