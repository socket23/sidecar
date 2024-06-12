use crate::chunking::text_document::Range;

/// Grab the file contents above, below and in the selection
pub fn split_file_content_into_parts(
    file_content: &str,
    selection_range: &Range,
) -> (Option<String>, Option<String>, String) {
    let lines = file_content
        .lines()
        .enumerate()
        .into_iter()
        .map(|(idx, line)| (idx as i64, line.to_owned()))
        .collect::<Vec<_>>();

    let start_line = selection_range.start_line() as i64;
    let end_line = selection_range.end_line() as i64;
    let above: Option<String>;
    if start_line == 0 {
        above = None;
    } else {
        let above_lines = lines
            .iter()
            .take_while(|(idx, _line)| idx < &start_line)
            .map(|(_, line)| line.to_owned())
            .collect::<Vec<_>>()
            .join("\n");
        above = Some(above_lines.to_owned());
    }

    // now we generate the section in the selection
    let selection_range = lines
        .iter()
        .skip_while(|(idx, _)| idx < &start_line)
        .take_while(|(idx, _)| idx <= &end_line)
        .map(|(_, line)| line.to_owned())
        .collect::<Vec<_>>()
        .join("\n");

    let below: Option<String>;
    if end_line >= lines.len() as i64 {
        below = None;
    } else {
        let below_lines = lines
            .iter()
            .skip_while(|(idx, _)| idx <= &end_line)
            .map(|(_, line)| line.to_owned())
            .collect::<Vec<_>>()
            .join("\n");
        below = Some(below_lines)
    }

    (above, below, selection_range)
}

/// Find the symbol in the line now
/// our home fed needle in haystack which works on character level instead
/// of byte level
/// This returns the last character position where the needle is contained in
/// the haystack
pub fn find_needle_position(haystack: &str, needle: &str) -> Option<usize> {
    let haystack_char_indices: Vec<_> = haystack.char_indices().collect();
    haystack.rfind(needle).map(|byte_pos| {
        haystack_char_indices
            .iter()
            .position(|(b, _)| *b == byte_pos)
            .unwrap()
            + needle.chars().count()
            - 1
    })
}
