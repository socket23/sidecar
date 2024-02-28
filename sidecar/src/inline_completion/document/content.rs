//! We keep track of the document lines properly, so we can get data about which lines have been
//! edited and which are not changed, this way we can know which lines to keep track of

use std::{collections::HashSet, sync::Arc};

use regex::Regex;
use tree_sitter::Tree;

use crate::chunking::{
    editor_parsing::EditorParsing,
    text_document::{Position, Range},
};

/// This contains the bag of words for the given snippets and it uses a custom
/// tokenizer to extract the words from the code
#[derive(Debug)]
pub struct BagOfWords {
    words: HashSet<String>,
    snippet: String,
}

impl BagOfWords {
    pub fn new(snippet: String) -> Self {
        let bag_of_words = BagOfWords::tokenize_call(snippet.as_str());
        BagOfWords {
            words: bag_of_words,
            snippet,
        }
    }

    fn check_valid_token(token: &str) -> bool {
        token.len() > 1
    }

    fn tokenize_call(code: &str) -> HashSet<String> {
        let re = Regex::new(r"\b\w+\b").unwrap();
        let mut valid_tokens: HashSet<String> = Default::default();

        for m in re.find_iter(code) {
            let text = m.as_str();

            if text.contains('_') {
                // snake_case
                let parts: Vec<&str> = text.split('_').collect();
                for part in parts {
                    if BagOfWords::check_valid_token(part) {
                        valid_tokens.insert(part.to_lowercase());
                    }
                }
            } else if text.chars().any(|c| c.is_uppercase()) {
                // PascalCase and camelCase
                let camel_re = Regex::new(r"[A-Z][a-z]+|[a-z]+|[A-Z]+(?=[A-Z]|$)").unwrap();
                let parts: Vec<&str> = camel_re.find_iter(text).map(|mat| mat.as_str()).collect();
                for part in parts {
                    if BagOfWords::check_valid_token(part) {
                        valid_tokens.insert(part.to_lowercase());
                    }
                }
            } else {
                if BagOfWords::check_valid_token(text) {
                    valid_tokens.insert(text.to_lowercase());
                }
            }
        }

        // Now we want to create the bigrams and the tigrams from these tokens
        // and have them stored too, so we can process them
        valid_tokens
    }
}

/// Keeps track of the lines which have been added and edited into the code
/// Note: This does not keep track of the lines which have been removed
#[derive(Clone, Debug)]
pub enum DocumentLineStatus {
    Edited,
    Unedited,
}

pub struct DocumentLine {
    line_status: DocumentLineStatus,
    content: String,
}

impl DocumentLine {
    pub fn line_status(&self) -> DocumentLineStatus {
        self.line_status.clone()
    }

    pub fn is_edited(&self) -> bool {
        matches!(self.line_status, DocumentLineStatus::Edited)
    }

    pub fn is_unedited(&self) -> bool {
        matches!(self.line_status, DocumentLineStatus::Unedited)
    }
}

pub struct DocumentEditLines {
    lines: Vec<DocumentLine>,
    file_path: String,
    language: String,
    // What snippets are in the document
    // Some things we should take care of:
    // when providing context to the inline autocomplete we want to make sure that
    // the private methods are not shown (cause they are not necessary)
    // when showing snippets for jaccard similarity, things are difference
    // we want to show the content for it no matter what
    // basically if its because of a symbol then we should only show the outline here
    // but if that's not the case, then its fine
    window_snippets: Vec<BagOfWords>,
    editor_parsing: Arc<EditorParsing>,
    tree: Option<Tree>,
}

impl DocumentEditLines {
    pub fn new(
        file_path: String,
        content: String,
        language: String,
        editor_parsing: Arc<EditorParsing>,
    ) -> DocumentEditLines {
        let mut document_lines = if content == "" {
            DocumentEditLines {
                lines: vec![DocumentLine {
                    line_status: DocumentLineStatus::Unedited,
                    content: "".to_string(),
                }],
                file_path,
                language,
                window_snippets: vec![],
                editor_parsing,
                tree: None,
            }
        } else {
            let lines = content
                .lines()
                .map(|line_content| DocumentLine {
                    line_status: DocumentLineStatus::Unedited,
                    content: line_content.to_string(),
                })
                .collect::<Vec<_>>();
            DocumentEditLines {
                lines,
                file_path,
                language,
                window_snippets: vec![],
                editor_parsing,
                tree: None,
            }
        };
        document_lines.set_tree();
        document_lines
    }

    fn set_tree(&mut self) {
        if let Some(language_config) = self.editor_parsing.for_file_path(&self.file_path) {
            let tree = language_config.get_tree_sitter_tree(self.get_content().as_bytes());
            self.tree = tree;
        }
    }

    pub fn get_content(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.content.clone())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn remove_range(&mut self, range: Range) {
        let start_line = range.start_line();
        let start_column = range.start_column();
        let end_line = range.end_line();
        let end_column = range.end_column();
        if start_line == end_line {
            if start_column == end_column {
                return;
            } else {
                // we get the line at this line number and remove the content between the start and end columns
                let line = self.lines.get_mut(start_line).unwrap();
                let start_index = start_column;
                let end_index = end_column;
                let mut characters = line.content.chars().collect::<Vec<_>>();
                let start_index = start_index as usize;
                let end_index = end_index as usize;
                characters.drain(start_index..end_index + 1);
                line.content = characters.into_iter().collect();
            }
        } else {
            // This is a more complicated case
            // we handle it by the following ways:
            // - handle the start line and keep the prefix required
            // - handle the end line and keep the suffix as required
            // - remove the lines in between
            // - merge the prefix and suffix of the start and end lines

            // get the start of line prefix
            let start_line_characters = self.lines[start_line].content.chars().collect::<Vec<_>>();
            let start_line_prefix = start_line_characters[..start_column as usize].to_owned();
            // get the end of line suffix
            let end_line_characters = self.lines[end_line].content.chars().collect::<Vec<_>>();
            let end_line_suffix = end_line_characters[end_column..].to_owned();
            {
                let start_doc_line = self.lines.get_mut(start_line).unwrap();
                start_doc_line.content = start_line_prefix.into_iter().collect::<String>()
                    + &end_line_suffix.into_iter().collect::<String>();
            }
            // remove the lines in between the start line and the end line
            self.lines.drain(start_line + 1..end_line + 1);
            // remove the lines in between the start line and the end line
        }
    }

    fn insert_at_position(&mut self, position: Position, content: String) {
        // when we want to insert at the position so first we try to start appending it at the line number from the current column
        // position and also add the suffix which we have, this way we get the new lines which need to be inserted
        let line_content = self.lines[position.line()].content.to_owned();
        let characters = line_content.chars().into_iter().collect::<Vec<_>>();
        // get the prefix right before the column position
        let prefix = characters[..position.column() as usize]
            .to_owned()
            .into_iter()
            .collect::<String>();
        // get the suffix right after the column position
        let suffix = characters[position.column() as usize..]
            .to_owned()
            .into_iter()
            .collect::<String>();
        // the new content here is the prefix + content + suffix
        let new_content = format!("{}{}{}", prefix.to_owned(), content, suffix);
        // now we get the new lines which need to be inserted
        let new_lines = new_content.lines().map(|line| DocumentLine {
            line_status: DocumentLineStatus::Edited,
            content: line.to_owned(),
        });
        // we also need to remove the line at the current line number
        self.lines.remove(position.line());
        // now we add back the lines which need to be inserted
        self.lines
            .splice(position.line()..position.line(), new_lines);
    }

    fn snippets_using_sliding_window(&mut self, lines: Vec<String>) {
        // Maximum snippet size here is 50 lines and we want to generate the snippets using the lines
        let mut final_snippets = vec![];
        if lines.len() <= 50 {
            final_snippets.push(BagOfWords::new(lines.join("\n")));
        } else {
            for i in 0..(lines.len() - 50) {
                let mut current_lines = vec![];
                for j in 0..50 {
                    if i + j >= lines.len() {
                        break;
                    }
                    current_lines.push(lines[i + j].to_owned());
                }
                final_snippets.push(BagOfWords::new(current_lines.join("\n")));
            }
        }
        self.window_snippets = final_snippets;
    }

    fn generate_snippets(&mut self) {
        // generate the new tree sitter tree
        self.set_tree();

        let content = self.get_content();

        let source_code = content.as_bytes();
        // For generating the snippets we have to use the following tricks which might be useful
        // - we do not want to include imports (they are just noise)
        // - we want to provide the implementations of the functions and classes, these are necessary
        // - can a stupid sliding window here work as we want?
        let language_config = self.editor_parsing.for_file_path(&self.file_path);
        let mut exlcuded_lines_import: HashSet<usize> = Default::default();
        match (language_config, self.tree.as_ref()) {
            (Some(language_config), Some(tree)) => {
                let excluded_ranges = language_config.get_import_ranges(tree, source_code);
                excluded_ranges.into_iter().for_each(|range| {
                    let start_line = range.start_line();
                    let end_line = range.end_line();
                    // Now we grab all the lines between start and end line
                    for i in start_line..end_line + 1 {
                        exlcuded_lines_import.insert(i);
                    }
                });
            }
            _ => {}
        }

        // we check what different types of constructs we have in the tree, and then only exclude the
        // import lines which are not convered by any of the other constructs
        if let (Some(language_config), Some(tree)) = (language_config, self.tree.as_ref()) {
            // remove the lines which are covered by the functions, since these are part of function bodies
            language_config
                .capture_function_data_with_tree(source_code, tree)
                .into_iter()
                .for_each(|function_data| {
                    let range = function_data.range();
                    for line in range.start_line()..range.end_line() + 1 {
                        exlcuded_lines_import.remove(&line);
                    }
                });
        }

        // now we create the new file content after removing the import lines
        let mut filtered_lines = vec![];
        for (i, line) in self.lines.iter().enumerate() {
            if !exlcuded_lines_import.contains(&i) {
                filtered_lines.push(line.content.to_owned());
            }
        }

        // after filtered content we have to grab the sliding window context, we generate the windows
        // we have some interesting things we can do while generating the code context
        self.snippets_using_sliding_window(filtered_lines);
    }

    // If the contents have changed, we need to mark the new lines which have changed
    pub fn content_change(&mut self, range: Range, new_content: String) {
        // First we remove the content at the range which is changing
        self.remove_range(range);
        // Then we insert the new content at the range
        self.insert_at_position(range.start_position(), new_content);
        // We want to get the code snippets here and make sure that the edited code snippets
        // are together when creating the window
        self.generate_snippets();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::chunking::{
        editor_parsing::EditorParsing,
        text_document::{Position, Range},
    };

    use super::DocumentEditLines;

    #[test]
    fn test_remove_range_works_as_expected() {
        let editor_parsing = Arc::new(EditorParsing::default());
        let mut document = DocumentEditLines::new(
            "".to_owned(),
            r#"FIRST LINE
SECOND LINE
THIRD LINE
FOURTH LINE
FIFTH LINE 🫡
SIXTH LINE 🫡🚀"#
                .to_owned(),
            "".to_owned(),
            editor_parsing,
        );
        let range = Range::new(Position::new(4, 0, 0), Position::new(5, 0, 0));
        document.remove_range(range);
        let updated_content = document.get_content();
        assert_eq!(
            updated_content,
            r#"FIRST LINE
SECOND LINE
THIRD LINE
FOURTH LINE
SIXTH LINE 🫡🚀"#
        );
    }

    #[test]
    fn test_remove_range_empty_works() {
        let editor_parsing = Arc::new(EditorParsing::default());
        let mut document = DocumentEditLines::new(
            "".to_owned(),
            r#"SOMETHING"#.to_owned(),
            "".to_owned(),
            editor_parsing,
        );
        let range = Range::new(Position::new(0, 0, 0), Position::new(0, 0, 0));
        document.remove_range(range);
        let updated_content = document.get_content();
        assert_eq!(updated_content, "SOMETHING");
    }

    #[test]
    fn test_insert_at_position_works_as_expected() {
        let editor_parsing = Arc::new(EditorParsing::default());
        let mut document = DocumentEditLines::new(
            "".to_owned(),
            r#"FIRST LINE
SECOND LINE
THIRD LINE
🫡🫡🫡🫡
FIFTH LINE 🫡
SIXTH LINE 🫡🚀"#
                .to_owned(),
            "".to_owned(),
            editor_parsing,
        );
        let position = Position::new(3, 1, 0);
        document.insert_at_position(position, "🚀🚀🚀".to_owned());
        let updated_content = document.get_content();
        assert_eq!(
            updated_content,
            r#"FIRST LINE
SECOND LINE
THIRD LINE
🫡🚀🚀🚀
🫡🫡🫡
        );
        let position = Position::new(3, 1, 0);
        document.insert_at_position(position, "🚀🚀🚀\n🪨🪨".to_owned());
        let updated_content = document.get_content();
        assert_eq!(
            updated_content,
            r#"FIRST LINE
SECOND LINE
THIRD LINE
🫡🚀🚀🚀
🪨🪨🫡🫡🫡
FIFTH LINE 🫡
SIXTH LINE 🫡🚀"#
        );
    }

    #[test]
    fn test_insert_on_empty_document_works() {
        let editor_parsing = Arc::new(EditorParsing::default());
        let mut document =
            DocumentEditLines::new("".to_owned(), "".to_owned(), "".to_owned(), editor_parsing);
        let position = Position::new(0, 0, 0);
        document.insert_at_position(position, "SOMETHING".to_owned());
        let updated_content = document.get_content();
        assert_eq!(updated_content, "SOMETHING");
    }

    #[test]
    fn test_removing_all_content() {
        let editor_parsing = Arc::new(EditorParsing::default());
        let mut document = DocumentEditLines::new(
            "".to_owned(),
            r#"FIRST LINE
SECOND LINE
THIRD LINE
🫡🫡🫡🫡
FIFTH LINE 🫡
SIXTH LINE 🫡🚀"#
                .to_owned(),
            "".to_owned(),
            editor_parsing,
        );
        let range = Range::new(Position::new(0, 0, 0), Position::new(5, 13, 0));
        document.remove_range(range);
        let updated_content = document.get_content();
        assert_eq!(updated_content, "");
    }
}
