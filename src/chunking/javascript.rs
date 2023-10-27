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
    }
}
