use std::collections::HashMap;

use gix::index::extension::end_of_index_entry;

/// Context trimming helps us reduce the context required before we pass it to a LLM, for now
/// we will try to build this up as many hurestics adn reducing the blast radius of all the things
/// which are needed for LLM.
/// Few ideas:
/// - de-duplicate the code snippets (and merge the precise locations together)
/// - ask the LLM to go through the data using the query and figure out if the context is indeed required
/// - and finally once we have all the context we can just ask the LLM to answer
use crate::{repo::types::RepoRef, webserver::agent::PreciseContext};

use super::agent::{CurrentViewPort, CursorPosition, DeepContextForView, Position};

#[derive(Debug, Clone)]
pub struct TrimmedContext {
    current_view_port: Option<CurrentViewPort>,
    current_cursor_position: Option<CursorPosition>,
    repo_ref: RepoRef,
    /// This is grouped here so we can just send the LLM data once for a given context and ask it
    /// to decide
    precise_context_map: HashMap<String, Vec<PreciseContext>>,
}

pub struct ViewPortContext {
    view_port_string: String,
    cursor_in_view_port: bool,
}

pub async fn trim_deep_context(context: DeepContextForView) -> anyhow::Result<TrimmedContext> {
    // grab the precis-context here and use a hashing function to only keep the non-duplicates
    let mut precise_context_map: HashMap<String, Vec<PreciseContext>> = Default::default();
    let current_view_port = context.current_view_port;
    let current_cursor_position = context.cursor_position;
    let repo_ref = context.repo_ref;
    context.precise_context.into_iter().for_each(|context| {
        let definition_context = &context.definition_snippet;
        // get a hashing function now and only keep them
        let hash = blake3::hash(definition_context.as_bytes());
        let hashed_string = hash.to_string();
        precise_context_map
            .entry(hashed_string)
            .or_default()
            .push(context.clone());
    });
    Ok(TrimmedContext {
        current_view_port,
        current_cursor_position,
        repo_ref,
        precise_context_map,
    })
}

// Now we are going to ask the LLM to decide which of the snippets are relevant to the user query
// this will be decided by first asking the LLM to decide trim out the context even more and give
// us the pointers
pub async fn create_viewport_context(
    view_port: CurrentViewPort,
    cursor_position: Option<CursorPosition>,
) -> String {
    // Here we are going to decorate it as a code span but add a few more details
    // like where the cursor position is etc
    // we want it to the be in the format of a code span but also take into
    // consideration the cursor position
    let current_file_path = view_port.fs_file_path;
    let start_position = view_port.start_position;
    // we also have the cursor position which we need to take into consideration
    // to format the view port context correctly, since the cursor will be most of
    // the times in the range of the view port, if its not we skip it
    // we format this as:
    // {line_number}: {text}
    // {line_number + 1}: {text}
    // ...
    let view_port_text = match cursor_position {
        Some(cursor_position) => {
            let (cursor_start_position, cursor_end_position) =
                get_first_and_last_position_for_cursor(
                    cursor_position.start_position,
                    cursor_position.end_position,
                );
            // Now that we have the cursor positions we need to embed
            // <cursor_position> and </cursor_position> in the right places
            let view_port_text = view_port
                .text_on_screen
                .lines()
                .into_iter()
                .enumerate()
                .map(|(index, line)| {
                    let line_number = start_position.line + index;
                    // edge case here where we have to insert both the start and
                    // end markers on the same line
                    let final_line = if line_number == cursor_start_position.line
                        && line_number == cursor_end_position.line
                    {
                        let cursor_start_char_position = cursor_start_position.character;
                        let cursor_end_char_position = cursor_end_position.character;
                        // split the line at these indices and then insert the string
                        // at the positions
                        let mut line_str = line.to_owned();
                        line_str.insert_str(cursor_start_char_position, "<cursor_position>");

                        // Since we have inserted a string at position1, the second position has shifted by the length of the inserted string
                        let adjusted_position2 =
                            cursor_end_char_position + "<cursor_position>".len();

                        // Insert the second XML tag at the adjusted position2
                        line_str.insert_str(adjusted_position2, "</cursor_position>");
                        line_str
                    } else if line_number == cursor_start_position.line {
                        // here we need to insert the start tag
                        let cursor_start_char_position = cursor_start_position.character;
                        let mut line_str = line.to_owned();
                        line_str.insert_str(cursor_start_char_position, "<cursor_position>");
                        line_str
                    } else if line_number == cursor_end_position.line {
                        // here we need to insert the end tag
                        let cursor_end_char_position = cursor_end_position.character;
                        let mut line_str = line.to_owned();

                        // Insert the second XML tag at the adjusted position2
                        line_str.insert_str(cursor_end_char_position, "</cursor_position>");
                        line_str
                    } else {
                        line.to_owned()
                    };
                    // Here we check if the line number matches with the start
                    // or the end
                    let line_number = format!("{}: ", line_number);
                    let line = format!("{}{}", line_number, final_line);
                    line
                })
                .collect::<Vec<_>>()
                .join("\n");
            view_port_text
        }
        None => view_port
            .text_on_screen
            .lines()
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                let line_number = start_position.line + index;
                let line_number = line_number.to_string();
                let line_number = format!("{}: ", line_number);
                let line = format!("{}{}", line_number, line);
                line
            })
            .collect::<Vec<_>>()
            .join("\n"),
    };
    format!("File path: {}", current_file_path) + "\n" + &view_port_text
}

fn get_first_and_last_position_for_cursor(
    start_position: Position,
    end_position: Position,
) -> (Position, Position) {
    // the logic here is to find the positions in the cursor which are in the first
    // or the end position
    // since from the IDE we get the anchor element and the active position
    // the start and end might be reversed, so we need to fix that here
    if start_position.line == end_position.line {
        if start_position.character <= end_position.character {
            (start_position, end_position)
        } else {
            (end_position, start_position)
        }
    } else if start_position.line < end_position.line {
        (start_position, end_position)
    } else {
        (end_position, start_position)
    }
}

#[cfg(test)]
mod tests {
    use crate::webserver::agent::{CurrentViewPort, CursorPosition, Position};

    use super::create_viewport_context;

    #[tokio::test]
    async fn test_view_port_context_no_cursor() {
        let current_view_port = CurrentViewPort {
            start_position: Position {
                line: 1,
                character: 0,
            },
            end_position: Position {
                line: 3,
                character: 0,
            },
            relative_path: "test.rs".to_owned(),
            fs_file_path: "/Users/testing/test.rs".to_owned(),
            text_on_screen: r#"pub struct CurrentViewPort {
    pub start_position: Position,
    pub end_position: Position,
    pub relative_path: String,
    pub fs_file_path: String,
    pub text_on_screen: String,
}"#
            .to_owned(),
        };

        let cursor_position = None;
        let result = create_viewport_context(current_view_port, cursor_position).await;
        let expected_result = r#"File path: /Users/testing/test.rs
1: pub struct CurrentViewPort {
2:     pub start_position: Position,
3:     pub end_position: Position,
4:     pub relative_path: String,
5:     pub fs_file_path: String,
6:     pub text_on_screen: String,
7: }"#;
        assert_eq!(expected_result, result);
    }

    #[tokio::test]
    async fn test_view_port_with_cursor() {
        let current_view_port = CurrentViewPort {
            start_position: Position {
                line: 1,
                character: 0,
            },
            end_position: Position {
                line: 3,
                character: 0,
            },
            relative_path: "test.rs".to_owned(),
            fs_file_path: "/Users/testing/test.rs".to_owned(),
            text_on_screen: r#"pub struct CurrentViewPort {
    pub start_position: Position,
    pub end_position: Position,
    pub relative_path: String,
    pub fs_file_path: String,
    pub text_on_screen: String,
}"#
            .to_owned(),
        };

        let cursor_position = Some(CursorPosition {
            start_position: Position {
                line: 1,
                character: 7,
            },
            end_position: Position {
                line: 4,
                character: 14,
            },
        });
        let result = create_viewport_context(current_view_port, cursor_position).await;
        let expected_result = r#"File path: /Users/testing/test.rs
1: pub str<cursor_position>uct CurrentViewPort {
2:     pub start_position: Position,
3:     pub end_position: Position,
4:     pub relati</cursor_position>ve_path: String,
5:     pub fs_file_path: String,
6:     pub text_on_screen: String,
7: }"#;
        assert_eq!(result, expected_result);
    }

    #[tokio::test]
    async fn test_view_port_cursor_on_same_line() {
        let current_view_port = CurrentViewPort {
            start_position: Position {
                line: 1,
                character: 0,
            },
            end_position: Position {
                line: 3,
                character: 0,
            },
            relative_path: "test.rs".to_owned(),
            fs_file_path: "/Users/testing/test.rs".to_owned(),
            text_on_screen: r#"pub struct CurrentViewPort {
    pub start_position: Position,
    pub end_position: Position,
    pub relative_path: String,
    pub fs_file_path: String,
    pub text_on_screen: String,
}"#
            .to_owned(),
        };
        let cursor_position = Some(CursorPosition {
            start_position: Position {
                line: 1,
                character: 7,
            },
            end_position: Position {
                line: 1,
                character: 7,
            },
        });

        let result = create_viewport_context(current_view_port, cursor_position).await;
        let expected_result = r#"File path: /Users/testing/test.rs
1: pub str<cursor_position></cursor_position>uct CurrentViewPort {
2:     pub start_position: Position,
3:     pub end_position: Position,
4:     pub relative_path: String,
5:     pub fs_file_path: String,
6:     pub text_on_screen: String,
7: }"#;
        assert_eq!(expected_result, result);
    }
}
