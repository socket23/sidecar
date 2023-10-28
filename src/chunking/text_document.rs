use crate::repo::types::RepoRef;

use super::languages::TSLanguageConfig;

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
}

impl TextDocument {
    pub fn get_content_buffer(&self) -> &str {
        &self.text
    }
}

// These are always 0 indexed
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
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
}

#[derive(Debug, Clone)]
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
        dbg!(source_code[range.start_byte..range.end_byte].to_owned());
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
