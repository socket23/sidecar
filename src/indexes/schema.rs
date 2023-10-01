use tantivy::schema::{
    BytesOptions, Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, FAST, STORED,
    STRING,
};

use crate::db::sqlite::SqlDb;

/// A schema for indexing all files and directories, linked to a
/// single repository on disk.
#[derive(Clone)]
pub struct File {
    pub schema: Schema,
    // pub(super) semantic: Option<Semantic>,
    /// Unique ID for the file in a repo
    pub unique_hash: Field,

    pub sql: SqlDb,

    /// Path to the root of the repo on disk
    pub repo_disk_path: Field,
    /// Path to the file, relative to the repo root
    pub relative_path: Field,

    /// Unique repo identifier, of the form:
    ///  local: local//path/to/repo
    /// github: github.com/org/repo
    pub repo_ref: Field,

    /// Indexed repo name, of the form:
    ///  local: repo
    /// github: github.com/org/repo
    pub repo_name: Field,

    pub content: Field,
    pub line_end_indices: Field,

    /// a flat list of every symbol's text, for searching, e.g.:
    /// ["File", "Repo", "worker"]
    pub symbols: Field,

    /// fast fields for scoring
    pub lang: Field,
    pub avg_line_length: Field,
    pub last_commit_unix_seconds: Field,

    /// fast byte versions of certain fields for collector-level filtering
    pub raw_content: Field,
    pub raw_repo_name: Field,
    pub raw_relative_path: Field,

    /// list of branches in which this file can be found
    pub branches: Field,

    /// Whether this entry is a file or a directory
    pub is_directory: Field,

    /// How many commits have been made to this file in last 2 weeks
    pub commit_frequency: Field,
}

impl File {
    pub fn new(sql: SqlDb) -> Self {
        let mut builder = tantivy::schema::SchemaBuilder::new();
        let trigram = TextOptions::default().set_stored().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("default")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );

        let unique_hash = builder.add_text_field("unique_hash", STRING | STORED);

        let repo_disk_path = builder.add_text_field("repo_disk_path", STRING);
        let repo_ref = builder.add_text_field("repo_ref", STRING | STORED);
        let repo_name = builder.add_text_field("repo_name", trigram.clone());
        let relative_path = builder.add_text_field("relative_path", trigram.clone());

        let content = builder.add_text_field("content", trigram.clone());
        let line_end_indices =
            builder.add_bytes_field("line_end_indices", BytesOptions::default().set_stored());

        let symbols = builder.add_text_field("symbols", trigram.clone());
        let symbol_locations =
            builder.add_bytes_field("symbol_locations", BytesOptions::default().set_stored());

        let branches = builder.add_text_field("branches", trigram);

        let lang = builder.add_bytes_field(
            "lang",
            BytesOptions::default().set_stored().set_indexed() | FAST,
        );
        let avg_line_length = builder.add_f64_field("line_length", FAST);
        let last_commit_unix_seconds = builder.add_i64_field("last_commit_unix_seconds", FAST);

        let raw_content = builder.add_bytes_field("raw_content", FAST);
        let raw_repo_name = builder.add_bytes_field("raw_repo_name", FAST);
        let raw_relative_path = builder.add_bytes_field("raw_relative_path", FAST);

        let is_directory = builder.add_bool_field("is_directory", FAST);
        let commit_frequency = builder.add_u64_field("commit_frequency", FAST);

        Self {
            sql,
            repo_disk_path,
            relative_path,
            unique_hash,
            repo_ref,
            repo_name,
            content,
            line_end_indices,
            symbols,
            lang,
            avg_line_length,
            last_commit_unix_seconds,
            schema: builder.build(),
            raw_content,
            raw_repo_name,
            raw_relative_path,
            branches,
            is_directory,
            commit_frequency,
        }
    }
}
