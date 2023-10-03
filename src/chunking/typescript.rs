use crate::chunking::languages::TSLanguageConfig;

pub fn typescript_language_config() -> TSLanguageConfig {
    TSLanguageConfig {
        language_ids: &["Typescript", "TSX"],
        file_extensions: &["ts", "tsx", "jsx", "mjs"],
        grammar: tree_sitter_typescript::language_tsx,
        namespaces: vec![
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
        .collect(),
    }
}
