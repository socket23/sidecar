use crate::chunking::languages::TSLanguageConfig;

pub fn javascript_language_config() -> TSLanguageConfig {
    TSLanguageConfig {
        language_ids: &["Javascript", "JSX", "javascript", "jsx"],
        file_extensions: &["js", "jsx"],
        grammar: tree_sitter_javascript::language,
        namespaces: vec![
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
        .collect(),
        documentation_query: vec!["((comment) @comment
        (#match? @comment \"^\\\\/\\\\*\\\\*\")) @docComment"
            .to_owned()],
        function_query: vec!["[
				(function
					name: (identifier)? @identifier
					body: (statement_block) @body)
				(function_declaration
					name: (identifier)? @identifier
					body: (statement_block) @body)
				(generator_function
					name: (identifier)? @identifier
					body: (statement_block) @body)
				(generator_function_declaration
					name: (identifier)? @identifier
					body: (statement_block) @body)
				(method_definition
					name: (property_identifier)? @identifier
					body: (statement_block) @body)
				(arrow_function
					body: (statement_block) @body)
			] @function"
            .to_owned()],
    }
}
