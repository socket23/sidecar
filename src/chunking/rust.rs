/// We want to parse the rust language here and provide the language config
/// for it
use crate::chunking::languages::TSLanguageConfig;

pub fn rust_language_config() -> TSLanguageConfig {
    TSLanguageConfig {
        language_ids: &["Rust"],
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
    }
}
