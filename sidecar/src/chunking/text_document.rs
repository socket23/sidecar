use crate::repo::types::RepoRef;

use super::{languages::TSLanguageConfig, types::FunctionInformation};

#[derive(Debug)]
pub struct TextDocument {
    text: String,
    repo_ref: RepoRef,
    fs_file_path: String,
    relative_path: String,
}

impl TextDocument {
    pub fn new(
        text: String,
        repo_ref: RepoRef,
        fs_file_path: String,
        relative_path: String,
    ) -> Self {
        Self {
            text,
            repo_ref,
            fs_file_path,
            relative_path,
        }
    }

    /// Extracts a substring from the text document based on the given range.
    ///
    /// # Arguments
    ///
    /// * `range` - The range indicating the start and end positions of the substring.
    ///
    /// # Returns
    ///
    /// The extracted substring as a `String`.
    pub fn from_range(&self, range: &Range) -> String {
        self.text[range.start_byte()..range.end_byte()].to_owned()
    }
}

impl TextDocument {
    pub fn get_content_buffer(&self) -> &str {
        &self.text
    }
}

// These are always 0 indexed
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, std::hash::Hash)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    line: usize,
    character: usize,
    byte_offset: usize,
}

impl Into<tree_sitter::Point> for Position {
    fn into(self) -> tree_sitter::Point {
        self.to_tree_sitter()
    }
}

impl Position {
    fn to_tree_sitter(&self) -> tree_sitter::Point {
        tree_sitter::Point::new(self.line, self.character)
    }

    pub fn from_tree_sitter_point(point: &tree_sitter::Point, byte_offset: usize) -> Self {
        Self {
            line: point.row,
            character: point.column,
            byte_offset,
        }
    }

    pub fn to_byte_offset(&self) -> usize {
        self.byte_offset
    }

    pub fn new(line: usize, character: usize, byte_offset: usize) -> Self {
        Self {
            line,
            character,
            byte_offset,
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.character
    }

    pub fn set_byte_offset(&mut self, byte_offset: usize) {
        self.byte_offset = byte_offset;
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq, std::hash::Hash)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    start_position: Position,
    end_position: Position,
}

impl Range {
    pub fn new(start_position: Position, end_position: Position) -> Self {
        Self {
            start_position,
            end_position,
        }
    }

    pub fn start_position(&self) -> Position {
        self.start_position.clone()
    }

    pub fn end_position(&self) -> Position {
        self.end_position.clone()
    }

    pub fn start_byte(&self) -> usize {
        self.start_position.byte_offset
    }

    pub fn end_byte(&self) -> usize {
        self.end_position.byte_offset
    }

    pub fn intersection_size(&self, other: &Range) -> usize {
        let start = self
            .start_position
            .byte_offset
            .max(other.start_position.byte_offset);
        let end = self
            .end_position
            .byte_offset
            .min(other.end_position.byte_offset);
        std::cmp::max(0, end as i64 - start as i64) as usize
    }

    pub fn len(&self) -> usize {
        self.end_position.byte_offset - self.start_position.byte_offset
    }

    pub fn to_tree_sitter_range(&self) -> tree_sitter::Range {
        tree_sitter::Range {
            start_byte: self.start_position.byte_offset,
            end_byte: self.end_position.byte_offset,
            start_point: self.start_position.to_tree_sitter(),
            end_point: self.end_position.to_tree_sitter(),
        }
    }

    pub fn for_tree_node(node: &tree_sitter::Node) -> Self {
        let range = node.range();
        Self {
            start_position: Position {
                line: range.start_point.row,
                character: range.start_point.column,
                byte_offset: range.start_byte,
            },
            end_position: Position {
                line: range.end_point.row,
                character: range.end_point.column,
                byte_offset: range.end_byte,
            },
        }
    }

    pub fn is_contained(&self, other: &Self) -> bool {
        self.start_position.byte_offset <= other.start_position.byte_offset
            && self.end_position.byte_offset >= other.end_position.byte_offset
    }

    pub fn guard_large_expansion(selection_range: Self, expanded_range: Self, size: usize) -> Self {
        let start_line_difference =
            if selection_range.start_position().line() > expanded_range.start_position().line() {
                selection_range.start_position().line() - expanded_range.start_position().line()
            } else {
                expanded_range.start_position().line() - selection_range.start_position().line()
            };
        let end_line_difference =
            if selection_range.end_position().line() > expanded_range.end_position().line() {
                selection_range.end_position().line() - expanded_range.end_position().line()
            } else {
                expanded_range.end_position().line() - selection_range.end_position().line()
            };
        if (start_line_difference + end_line_difference) > 30 {
            // we are going to return the selection range here
            return selection_range.clone();
        } else {
            return expanded_range.clone();
        }
    }
}

#[derive(Debug, Clone)]
pub enum DocumentSymbolKind {
    Function,
    Class,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentSymbol {
    pub name: Option<String>,
    pub start_position: Position,
    pub end_position: Position,
    pub kind: Option<String>,
    pub code: String,
}

impl DocumentSymbol {
    pub fn for_edit(start_position: Position, end_position: Position) -> Self {
        Self {
            name: None,
            start_position,
            end_position,
            kind: None,
            // We send a placeholder for edit here
            code: "edit".to_owned(),
        }
    }
}

impl DocumentSymbol {
    fn get_node_matching<'a>(
        tree_cursor: &mut tree_sitter::TreeCursor<'a>,
        node: &tree_sitter::Node<'a>,
        regex: regex::Regex,
        source_code: &str,
    ) -> Option<tree_sitter::Node<'a>> {
        node.children(tree_cursor)
            .find(|node| regex.is_match(node.kind()))
    }

    fn get_identifier_node<'a>(
        node: &tree_sitter::Node<'a>,
        cursor: &mut tree_sitter::TreeCursor<'a>,
        second_cursor: &mut tree_sitter::TreeCursor<'a>,
        third_cursor: &mut tree_sitter::TreeCursor<'a>,
        language_config: &TSLanguageConfig,
        source_code: &'a str,
    ) -> Option<tree_sitter::Node<'a>> {
        match language_config
            .language_ids
            .first()
            .expect("language_ids to be present")
            .to_lowercase()
            .as_ref()
        {
            "python" | "c_sharp" | "ruby" | "rust" => DocumentSymbol::get_node_matching(
                cursor,
                node,
                regex::Regex::new("identifier").expect("regex to build"),
                source_code,
            ),
            "golang" => {
                let regex_matcher = regex::Regex::new("identifier").unwrap();
                let children =
                    DocumentSymbol::get_node_matching(cursor, node, regex_matcher, source_code);
                if let Some(children) = children {
                    return Some(children);
                } else {
                    let regex_matcher = regex::Regex::new("spec").unwrap();
                    if let Some(spec) = node
                        .children(second_cursor)
                        .find(|node| regex_matcher.is_match(node.kind()))
                    {
                        let regex_matcher = regex::Regex::new("identifier").unwrap();
                        return spec
                            .children(third_cursor)
                            .find(|node| regex_matcher.is_match(node.kind()));
                    } else {
                        None
                    }
                }
            }
            "javascript" | "javascript-react" | "typescript" | "typescript-react" | "cpp"
            | "java" => {
                let regex_matcher = regex::Regex::new("identifier").unwrap();
                let children =
                    DocumentSymbol::get_node_matching(cursor, node, regex_matcher, source_code);
                if let Some(children) = children {
                    return Some(children);
                } else {
                    let regex_matcher = regex::Regex::new("declarator").unwrap();
                    if let Some(spec) = node
                        .children(second_cursor)
                        .find(|node| regex_matcher.is_match(node.kind()))
                    {
                        let regex_matcher = regex::Regex::new("identifier").unwrap();
                        return spec
                            .children(third_cursor)
                            .find(|node| regex_matcher.is_match(node.kind()));
                    } else {
                        None
                    }
                }
            }
            _ => None,
        }
    }

    pub fn from_tree_node(
        tree_node: &tree_sitter::Node<'_>,
        language_config: &TSLanguageConfig,
        source_code: &str,
    ) -> Option<DocumentSymbol> {
        let mut walker = tree_node.walk();
        let mut second_walker = tree_node.walk();
        let mut third_walker = tree_node.walk();
        let range = tree_node.range();
        let start_position = Position {
            line: range.start_point.row,
            character: range.start_point.column,
            byte_offset: range.start_byte,
        };
        let end_position = Position {
            line: range.end_point.row,
            character: range.end_point.column,
            byte_offset: range.end_byte,
        };
        let identifier_node = DocumentSymbol::get_identifier_node(
            tree_node,
            &mut walker,
            &mut second_walker,
            &mut third_walker,
            language_config,
            source_code,
        );
        if let Some(identifier_node) = identifier_node {
            let kind = identifier_node.kind().to_owned();
            // We get a proper name for the identifier here so we can just use
            // thats
            let name =
                source_code[identifier_node.start_byte()..identifier_node.end_byte()].to_owned();
            Some(DocumentSymbol {
                name: Some(name),
                start_position,
                end_position,
                kind: Some(kind),
                code: source_code[tree_node.start_byte()..tree_node.end_byte()].to_owned(),
            })
        } else {
            Some(DocumentSymbol {
                name: None,
                start_position,
                end_position,
                kind: None,
                code: source_code[tree_node.start_byte()..tree_node.end_byte()].to_owned(),
            })
        }
    }
}

#[derive(Debug)]
pub struct OutlineForRange {
    above: String,
    below: String,
}

impl OutlineForRange {
    pub fn new(above: String, below: String) -> Self {
        Self { above, below }
    }

    pub fn get_tuple(self: Self) -> (String, String) {
        (self.above, self.below)
    }

    pub fn above(&self) -> &str {
        &self.above
    }

    pub fn below(&self) -> &str {
        &self.below
    }

    pub fn generate_outline_for_range(
        function_bodies: Vec<FunctionInformation>,
        range_expanded_to_function: Range,
        language: &str,
        source_code: Vec<u8>,
    ) -> Self {
        // Now we try to see if we can expand properly
        let mut terminator = "".to_owned();
        if language == "typescript" {
            terminator = ";".to_owned();
        }

        // we only keep the function bodies which are not too far away from
        // the range we are interested in selecting
        let filtered_function_bodies: Vec<_> = function_bodies
            .to_vec()
            .into_iter()
            .filter_map(|function_body| {
                let fn_body_end_line = function_body.range().end_position().line();
                let fn_body_start_line = function_body.range().start_position().line();
                let range_start_line = range_expanded_to_function.start_position().line();
                let range_end_line = range_expanded_to_function.end_position().line();
                if fn_body_end_line < range_start_line {
                    if range_start_line - fn_body_start_line > 50 {
                        Some(function_body)
                    } else {
                        None
                    }
                } else if fn_body_start_line > range_end_line {
                    if fn_body_end_line - range_end_line > 50 {
                        Some(function_body)
                    } else {
                        None
                    }
                } else {
                    Some(function_body)
                }
            })
            .collect();

        fn build_outline(
            source_code: Vec<u8>,
            function_bodies: Vec<FunctionInformation>,
            range: Range,
            terminator: &str,
        ) -> OutlineForRange {
            let mut current_index = 0;
            let mut outline_above = "".to_owned();
            let mut end_of_range = range.end_byte();
            let mut outline_below = "".to_owned();

            for function_body in function_bodies.iter() {
                if function_body.range().end_byte() < range.start_byte() {
                    outline_above += &String::from_utf8(
                        source_code
                            .get(current_index..function_body.range().start_byte())
                            .expect("to not fail")
                            .to_vec(),
                    )
                    .expect("ut8 errors to not happen");
                    outline_above += terminator;
                    current_index = function_body.range().end_byte();
                } else if function_body.range().start_byte() > range.end_byte() {
                    outline_below += &String::from_utf8(
                        source_code
                            .get(end_of_range..function_body.range().start_byte())
                            .expect("to not fail")
                            .to_vec(),
                    )
                    .expect("ut8 to not fail");
                    outline_below += terminator;
                    end_of_range = function_body.range().end_byte();
                } else {
                    continue;
                }
            }
            outline_above += &String::from_utf8(
                source_code
                    .get(current_index..range.start_byte())
                    .expect("to not fail")
                    .to_vec(),
            )
            .expect("ut8 to not fail");
            outline_below += &String::from_utf8(
                source_code
                    .get(end_of_range..source_code.len())
                    .expect("to not fail")
                    .to_vec(),
            )
            .expect("ut8 to not fail");
            OutlineForRange {
                above: outline_above,
                below: outline_below,
            }
        }
        build_outline(
            source_code,
            filtered_function_bodies,
            range_expanded_to_function,
            &terminator,
        )
    }
}
