use crate::{
    chunking::text_document::{Position, Range},
    inline_completion::types::InLineCompletionError,
};

/// Different kinds of completions we can have

#[derive(Debug, Clone)]
pub struct DocumentLines {
    lines: Vec<(i64, String)>,
    line_start_position: Vec<Position>,
    line_end_position: Vec<Position>,
}

impl DocumentLines {
    pub fn new(
        lines: Vec<(i64, String)>,
        line_start_position: Vec<Position>,
        line_end_position: Vec<Position>,
    ) -> Self {
        Self {
            lines,
            line_start_position,
            line_end_position,
        }
    }

    pub fn prefix_at_line(&self, position: Position) -> Result<String, InLineCompletionError> {
        let line_number = position.line();
        if line_number >= self.lines.len() {
            return Err(InLineCompletionError::PrefixNotFound);
        }
        let line = &self.lines[line_number];
        // Now only get the prefix for this from the current line
        let line_prefix = line.1[0..position.column() as usize].to_owned();
        Ok(line_prefix)
    }

    pub fn from_file_content(content: &str) -> Self {
        let mut byte_offset = 0;
        let lines: Vec<_> = content
            .lines()
            .enumerate()
            .map(|(_, line)| {
                // so here we will store the start and end byte position since we can
                // literally count the content size of the line and maintain
                // a running total of things
                let start = byte_offset;
                byte_offset += line.len();
                let end = byte_offset;
                byte_offset += 1; // for the newline
                (start, end)
            })
            .collect();
        let line_start_position: Vec<_> = content
            .lines()
            .enumerate()
            // the first entry is the start position offset, the second is the suffix
            .map(|(idx, _)| Position::new(idx, 0, lines[idx].0))
            .collect();
        let line_end_position: Vec<_> = content
            .lines()
            .enumerate()
            .map(|(idx, line)| Position::new(idx, line.chars().count(), lines[idx].1))
            .collect();
        Self::new(
            content
                .lines()
                .enumerate()
                .map(|(idx, line)| (idx as i64, line.to_string().to_owned()))
                .collect::<Vec<_>>(),
            line_start_position,
            line_end_position,
        )
    }

    pub fn get_line(&self, line_number: usize) -> &str {
        &self.lines[line_number].1
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn start_position_at_line(&self, line_number: usize) -> Position {
        self.line_start_position[line_number]
    }

    pub fn end_position_at_line(&self, line_number: usize) -> Position {
        self.line_end_position[line_number]
    }
}

#[derive(Debug, Clone)]
pub struct CodeSelection {
    range: Range,
    file_path: String,
    content: String,
}

impl CodeSelection {
    pub fn new(range: Range, file_path: String, content: String) -> Self {
        Self {
            range,
            file_path,
            content,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

pub enum CompletionContext {
    CurrentFile,
}

#[derive(Debug, Clone)]
pub struct CurrentFilePrefixSuffix {
    pub prefix: CodeSelection,
    pub suffix: CodeSelection,
}

impl CurrentFilePrefixSuffix {
    pub fn new(prefix: CodeSelection, suffix: CodeSelection) -> Self {
        Self { prefix, suffix }
    }
}
