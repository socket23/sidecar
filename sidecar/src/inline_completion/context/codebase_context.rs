use std::sync::Arc;

use llm_client::{
    clients::types::LLMType,
    tokenizer::tokenizer::{LLMTokenizer, LLMTokenizerInput},
};

use crate::{
    chunking::{editor_parsing::EditorParsing, text_document::Position},
    inline_completion::{
        document::content::SnippetInformation, symbols_tracker::SymbolTrackerInline,
        types::InLineCompletionError,
    },
};

/// Creates the codebase context which we want to use
/// for generating inline-completions
pub struct CodeBaseContext {
    tokenizer: Arc<LLMTokenizer>,
    llm_type: LLMType,
    file_path: String,
    file_content: String,
    cursor_position: Position,
    symbol_tracker: Arc<SymbolTrackerInline>,
    editor_parsing: Arc<EditorParsing>,
}

// .lines()

pub enum CodebaseContextString {
    TruncatedToLimit(String, i64),
    UnableToTruncate,
}

impl CodebaseContextString {
    pub fn get_prefix_with_tokens(self) -> Option<(String, i64)> {
        match self {
            CodebaseContextString::TruncatedToLimit(prefix, used_tokens) => {
                Some((prefix, used_tokens))
            }
            CodebaseContextString::UnableToTruncate => None,
        }
    }
}

impl CodeBaseContext {
    pub fn new(
        tokenizer: Arc<LLMTokenizer>,
        llm_type: LLMType,
        file_path: String,
        file_content: String,
        cursor_position: Position,
        symbol_tracker: Arc<SymbolTrackerInline>,
        editor_parsing: Arc<EditorParsing>,
    ) -> Self {
        Self {
            tokenizer,
            llm_type,
            file_path,
            file_content,
            cursor_position,
            symbol_tracker,
            editor_parsing,
        }
    }

    pub fn get_context_window_from_current_file(&self) -> String {
        let current_line = self.cursor_position.line();
        let lines = self.file_content.lines().collect::<Vec<_>>();
        let start_line = if current_line >= 50 {
            current_line - 50
        } else {
            0
        };
        let end_line = current_line;
        let context_lines = lines[start_line..end_line].join("\n");
        context_lines
    }

    pub async fn generate_context(
        &self,
        token_limit: usize,
    ) -> Result<CodebaseContextString, InLineCompletionError> {
        let mut used_tokens_for_prefix = token_limit;
        let language_config = self.editor_parsing.for_file_path(&self.file_path).ok_or(
            InLineCompletionError::LanguageNotSupported("not_supported".to_owned()),
        )?;
        let current_window_context = self.get_context_window_from_current_file();
        // Now we try to get the context from the symbol tracker
        let history_files = self.symbol_tracker.get_document_history().await;
        // since these history files are sorted in the order of priority, we can
        // safely assume that the first one is the most recent one

        let mut running_context: Vec<String> = vec![];
        // TODO(skcd): hate hate hate, but there's a mutex lock so this is fine ❤️‍🔥
        for history_file in history_files.into_iter() {
            let snippet_information = self
                .symbol_tracker
                .get_document_lines(&history_file, &current_window_context)
                .await;

            if let Some(snippet_information) = snippet_information {
                let merged_snippets = SnippetInformation::coelace_snippets(snippet_information);
                let code_context = merged_snippets
                    .into_iter()
                    .map(|snippet| {
                        snippet
                            .snippet()
                            .lines()
                            .into_iter()
                            .map(|line| format!("{} {}", language_config.comment_prefix, line))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .collect::<Vec<_>>();
                let file_path_header =
                    format!("{} Path: {}", language_config.comment_prefix, history_file);
                let joined_code_context = code_context.join("\n\n");
                let prefix_context = format!("{}\n{}", file_path_header, joined_code_context);
                running_context.push(prefix_context);

                let context_for_token_count = running_context.join("\n\n");
                used_tokens_for_prefix = self.tokenizer.count_tokens(
                    &self.llm_type,
                    LLMTokenizerInput::Prompt(context_for_token_count.to_owned()),
                )?;
                if token_limit > used_tokens_for_prefix {
                    return Ok(CodebaseContextString::TruncatedToLimit(
                        context_for_token_count,
                        used_tokens_for_prefix as i64,
                    ));
                }
            }

            // Now we need to deduplicate and merge the snippets which are overlapping
        }
        let prefix_context = running_context.join("\n\n");
        let used_tokens_for_prefix = self.tokenizer.count_tokens(
            &self.llm_type,
            LLMTokenizerInput::Prompt(prefix_context.to_owned()),
        )?;
        Ok(CodebaseContextString::TruncatedToLimit(
            prefix_context,
            used_tokens_for_prefix as i64,
        ))
    }
}
