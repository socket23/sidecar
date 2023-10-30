use regex::Regex;

use crate::chunking::text_document::{OutlineForRange, Range};

#[derive(Debug)]
pub struct ContextWindowTracker {
    token_limit: usize,
    total_tokens: usize,
}

impl ContextWindowTracker {
    pub fn new(token_limit: usize) -> Self {
        Self {
            token_limit,
            total_tokens: 0,
        }
    }

    pub fn add_tokens(&mut self, tokens: usize) {
        self.total_tokens += tokens;
    }

    pub fn tokens_remaining(&self) -> usize {
        self.token_limit - self.total_tokens
    }

    pub fn line_would_fit(&self, line: &str) -> bool {
        self.total_tokens + line.len() + 1 < self.token_limit
    }

    pub fn add_line(&mut self, line: &str) {
        self.total_tokens += line.len() + 1;
    }

    pub fn process_outlines(&mut self, generated_outline: OutlineForRange) -> OutlineForRange {
        // here we will process the outline again and try to generate it after making
        // sure that it fits in the limit
        let split_lines_regex = Regex::new(r"\r\n|\r|\n").unwrap();
        let lines_above: Vec<String> = split_lines_regex
            .split(&generated_outline.above())
            .map(|s| s.to_owned())
            .collect();
        let lines_below: Vec<String> = split_lines_regex
            .split(&generated_outline.below())
            .map(|s| s.to_owned())
            .collect();

        let mut processed_above = vec![];
        let mut processed_below = vec![];

        let mut try_add_above_line =
            |line: &str, context_manager: &mut ContextWindowTracker| -> bool {
                if context_manager.line_would_fit(line) {
                    context_manager.add_line(line);
                    processed_above.insert(0, line.to_owned());
                    return true;
                }
                false
            };

        let mut try_add_below_line =
            |line: &str, context_manager: &mut ContextWindowTracker| -> bool {
                if context_manager.line_would_fit(line) {
                    context_manager.add_line(line);
                    processed_below.push(line.to_owned());
                    return true;
                }
                false
            };

        let mut above_index: i64 = <i64>::try_from(lines_above.len() - 1).expect("to work");
        let mut below_index = 0;
        let mut can_add_above = true;
        let mut can_add_below = true;

        for index in 0..100 {
            if !can_add_above || (can_add_below && index % 4 == 3) {
                if below_index < lines_below.len()
                    && try_add_below_line(&lines_below[below_index], self)
                {
                    below_index += 1;
                } else {
                    can_add_below = false;
                }
            } else {
                if above_index >= 0
                    && try_add_above_line(
                        &lines_above[<usize>::try_from(above_index).expect("to work")],
                        self,
                    )
                {
                    above_index -= 1;
                } else {
                    can_add_above = false;
                }
            }
        }

        OutlineForRange::new(processed_above.join("\n"), processed_below.join("\n"))
    }
}

#[derive(Debug)]
pub struct ContextParserInLineEdit {
    language: String,
    unique_identifier: String,
    first_line_index: i64,
    last_line_index: i64,
    is_complete: bool,
    non_trim_whitespace_character_count: i64,
    start_marker: String,
    end_marker: String,
    // This is the lines coming from the source
    source_lines: Vec<String>,
    /// This is the lines we are going to use for the context
    lines: Vec<String>,
}

impl ContextParserInLineEdit {
    pub fn new(
        language: String,
        unique_identifier: String,
        lines_count: i64,
        source_lines: Vec<String>,
    ) -> Self {
        let comment_style = "//".to_owned();
        Self {
            language,
            unique_identifier: unique_identifier.to_owned(),
            first_line_index: lines_count,
            last_line_index: -1,
            is_complete: false,
            non_trim_whitespace_character_count: 0,
            // we also need to provide the comment style here, lets assume
            // that we are using //
            start_marker: format!("{} BEGIN: {}", &comment_style, unique_identifier),
            end_marker: format!("{} END: {}", &comment_style, unique_identifier),
            source_lines,
            lines: vec![],
        }
    }

    pub fn line_string(&self) -> String {
        self.lines.join("\n")
    }

    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    pub fn mark_complete(&mut self) {
        self.is_complete = true;
    }

    pub fn has_context(&self) -> bool {
        if self.lines.len() == 0 || self.non_trim_whitespace_character_count == 0 {
            false
        } else {
            !self.lines.is_empty()
        }
    }

    pub fn prepend_line(
        &mut self,
        line_index: usize,
        character_limit: &mut ContextWindowTracker,
    ) -> bool {
        let line_text = self.source_lines[line_index].to_owned();
        if !character_limit.line_would_fit(&line_text) {
            return false;
        }

        self.first_line_index = std::cmp::min(self.first_line_index, line_index as i64);
        self.last_line_index = std::cmp::max(self.last_line_index, line_index as i64);

        character_limit.add_line(&line_text);
        self.non_trim_whitespace_character_count += line_text.trim().len() as i64;
        self.lines.insert(0, line_text);

        true
    }

    pub fn append_line(
        &mut self,
        line_index: usize,
        character_limit: &mut ContextWindowTracker,
    ) -> bool {
        let line_text = self.source_lines[line_index].to_owned();
        if !character_limit.line_would_fit(&line_text) {
            return false;
        }

        self.first_line_index = std::cmp::min(self.first_line_index, line_index as i64);
        self.last_line_index = std::cmp::max(self.last_line_index, line_index as i64);

        character_limit.add_line(&line_text);
        self.non_trim_whitespace_character_count += line_text.trim().len() as i64;
        self.lines.push(line_text);

        true
    }

    pub fn trim(&mut self, range: Option<&Range>) {
        // now we can begin trimming it on a range if appropriate and then
        // do things properly
        let last_line_index = if let Some(range) = range.clone() {
            if self.last_line_index
                < range
                    .start_position()
                    .line()
                    .try_into()
                    .expect("usize to i64 not fail")
            {
                self.last_line_index
            } else {
                range
                    .start_position()
                    .line()
                    .try_into()
                    .expect("usize to i64 not fail")
            }
        } else {
            self.last_line_index
        };
        for _ in self.first_line_index..last_line_index {
            if self.lines.len() > 0 && self.lines[0].trim().len() == 0 {
                self.first_line_index += 1;
                self.lines.remove(0);
            }
        }

        let first_line_index = if let Some(range) = range {
            if self.first_line_index
                > range
                    .end_position()
                    .line()
                    .try_into()
                    .expect("usize to i64 not fail")
            {
                self.first_line_index
            } else {
                range
                    .end_position()
                    .line()
                    .try_into()
                    .expect("usize to i64 not fail")
            }
        } else {
            self.first_line_index
        };

        for _ in first_line_index..self.last_line_index {
            if self.lines.len() > 0 && self.lines[self.lines.len() - 1].trim().len() == 0 {
                self.last_line_index -= 1;
                self.lines.pop();
            }
        }
    }
}
