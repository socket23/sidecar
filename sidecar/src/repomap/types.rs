use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag {
    rel_fname: PathBuf,
    fname: PathBuf,
    line: usize,
    name: String,
    kind: TagKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagKind {
    Definition,
    Reference,
}