/// We want to parse the python language properly and the language config
/// for it
use crate::chunking::languages::TSLanguageConfig;

pub fn python_language_config() -> TSLanguageConfig {
    TSLanguageConfig {
        language_ids: &["Python", "python"],
        file_extensions: &["py"],
        grammar: tree_sitter_python::language,
        namespaces: vec!["class", "function", "parameter", "variable"]
            .into_iter()
            .map(|s| s.to_owned())
            .collect(),
        documentation_query: vec!["(expression_statement
                (string) @docComment"
            .to_owned()],
        function_query: vec![
            "[
                (function_definition
                    name: (identifier) @identifier
                    parameters: (parameters) @parameters
                    body: (block
                            (expression_statement (string))? @docstring) @body)
                (assignment
                    left: (identifier) @identifier
                    type: (type) @parameters
                    right: (lambda) @body)
            ] @function"
                .to_owned(),
            "(ERROR (\"def\" (identifier) (parameters))) @function".to_owned(),
        ],
        construct_types: vec!["module", "class_definition", "function_definition"]
            .into_iter()
            .map(|s| s.to_owned())
            .collect(),
        expression_statements: vec!["expression_statement".to_owned()],
    }
}
