use regex::Regex;

use crate::chunking::text_document::{Position, Range};

/// We are going to fix the range here based on the text document
/// following this convention because the byte offset from vscode
/// is different from the byte offset in rust
pub fn fix_vscode_range(range: Range, text_bytes: &[u8]) -> Range {
    // First we convert from the bytes to the string
    let mut fixed_range = range.clone();

    let start_position = fix_vscode_position(fixed_range.start_position(), text_bytes);
    let end_position = fix_vscode_position(fixed_range.end_position(), text_bytes);
    Range::new(start_position, end_position)
}

pub fn fix_vscode_position(mut position: Position, text_bytes: &[u8]) -> Position {
    let text_str = String::from_utf8(text_bytes.to_vec()).unwrap_or_default();
    // Now we have to split the text on the new lines
    let re = Regex::new(r"\r\n|\r|\n").unwrap();

    // Split the string using the regex pattern
    let lines: Vec<&str> = re.split(&text_str).collect();
    let position_byte_offset =
        line_column_to_byte_offset(lines.to_vec(), position.line(), position.column());
    if let Some(byte_offset) = position_byte_offset {
        position.set_byte_offset(byte_offset);
    }
    position
}

fn line_column_to_byte_offset(
    lines: Vec<&str>,
    target_line: usize,
    target_column: usize,
) -> Option<usize> {
    // Keep track of the current line and column in the input text
    let mut current_byte_offset = 0;

    for (index, line) in lines.iter().enumerate() {
        if index == target_line {
            let mut current_col = 0;

            // If the target column is at the beginning of the line
            if target_column == 0 {
                return Some(current_byte_offset);
            }

            for char in line.chars() {
                if current_col == target_column {
                    return Some(current_byte_offset);
                }
                current_byte_offset += char.len_utf8();
                current_col += 1;
            }

            // If the target column is exactly at the end of this line
            if current_col == target_column {
                return Some(current_byte_offset); // target_column is at the line break
            }

            // Column requested is beyond the current line length
            return None;
        }

        // Increment the byte offset by the length of the current line and its newline
        current_byte_offset += line.len() + "\n".len(); // add 1 for the newline character
    }

    // Line requested is beyond the input text line count
    None
}
