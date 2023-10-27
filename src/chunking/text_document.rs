use crate::repo::types::RepoRef;

use super::languages::TSLanguageConfig;

/// Contains information about the text document

#[derive(Debug, Clone)]
pub enum TextDocumentExtension {
    Markdown,
    Plaintext,
    Code,
    Cpp,
    Python,
    Rust,
    JavaScript,
}

#[derive(Debug)]
pub struct TextDocument {
    text: String,
    extension: TextDocumentExtension,
    repo_ref: RepoRef,
    fs_file_path: String,
    relative_path: String,
}

impl TextDocument {
    pub fn get_content_buffer(&self) -> &str {
        &self.text
    }
}

// These are always 0 indexed
#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
pub struct Range {
    start_position: Position,
    end_position: Position,
}

impl Range {
    pub fn intersection_size(&self, other: &Range) -> usize {
        let start = self
            .start_position
            .byte_offset
            .max(other.start_position.byte_offset);
        let end = self
            .end_position
            .byte_offset
            .min(other.end_position.byte_offset);
        std::cmp::max(0, end - start)
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
        Self {
            start_position: Position {
                line: node.start_position().row,
                character: node.start_position().column,
                byte_offset: node.start_byte(),
            },
            end_position: Position {
                line: node.end_position().row,
                character: node.end_position().column,
                byte_offset: node.end_byte(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum DocumentSymbolKind {
    Function,
    Class,
}

#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    pub name: String,
    pub start_position: Position,
    pub end_position: Position,
    pub kind: String,
}

impl DocumentSymbol {
    fn get_node_matching<'a>(
        tree_cursor: &mut tree_sitter::TreeCursor<'a>,
        node: &tree_sitter::Node<'a>,
        regex: regex::Regex,
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
            ),
            "golang" => {
                let regex_matcher = regex::Regex::new("identifier").unwrap();
                let children = DocumentSymbol::get_node_matching(cursor, node, regex_matcher);
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
                let children = DocumentSymbol::get_node_matching(cursor, node, regex_matcher);
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
        let identifier_node = DocumentSymbol::get_identifier_node(
            tree_node,
            &mut walker,
            &mut second_walker,
            &mut third_walker,
            language_config,
        );
        if let Some(identifier_node) = identifier_node {
            let start_position = Position {
                line: identifier_node.start_position().row,
                character: identifier_node.start_position().column,
                byte_offset: identifier_node.start_byte(),
            };
            let end_position = Position {
                line: identifier_node.end_position().row,
                character: identifier_node.end_position().column,
                byte_offset: identifier_node.end_byte(),
            };
            let kind = identifier_node.kind().to_owned();
            // This can fail but it shouldn't if this blows up we fatal bad
            let name = source_code[start_position.byte_offset..end_position.byte_offset].to_owned();
            Some(DocumentSymbol {
                name,
                start_position,
                end_position,
                kind,
            })
        } else {
            None
        }
    }
}
