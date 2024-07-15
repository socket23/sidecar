use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag {
    pub rel_fname: PathBuf,
    pub fname: PathBuf,
    pub line: usize,
    pub name: String,
    pub kind: TagKind,
}

impl Tag {
    pub fn new(
        rel_fname: PathBuf,
        fname: PathBuf,
        line: usize,
        name: String,
        kind: TagKind,
    ) -> Self {
        Self {
            rel_fname,
            fname,
            line,
            name,
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagKind {
    Definition,
    Reference,
}

/// An index structure for managing tags across multiple files.
pub struct TagIndex {
    /// Maps tag names to the set of file paths where the tag is defined.
    ///
    /// Useful for answering: "In which files is tag X defined?"
    pub defines: HashMap<String, HashSet<PathBuf>>,

    /// Maps tag names to a list of file paths where the tag is referenced.
    ///
    /// Allows duplicates to accommodate multiple references to the same definition.
    pub references: HashMap<String, Vec<PathBuf>>,

    /// Maps (file path, tag name) pairs to a set of tag definitions.
    ///
    /// Useful for answering: "What are the details of tag X in file Y?"
    pub definitions: HashMap<(PathBuf, String), HashSet<Tag>>,

    /// A set of commonly used tags across all files.
    pub common_tags: HashSet<String>,
}

impl TagIndex {
    pub fn new() -> Self {
        Self {
            defines: HashMap::new(),
            references: HashMap::new(),
            definitions: HashMap::new(),
            common_tags: HashSet::new(),
        }
    }

    pub fn add_tag(&mut self, tag: Tag, rel_path: PathBuf) {
        match tag.kind {
            TagKind::Definition => {
                self.defines
                    .entry(tag.name.clone())
                    .or_default()
                    .insert(rel_path.clone());
                self.definitions
                    .entry((rel_path.clone(), tag.name.clone()))
                    .or_default()
                    .insert(tag);
            }
            TagKind::Reference => {
                self.references
                    .entry(tag.name.clone())
                    .or_default()
                    .push(rel_path.clone());
            }
        }
    }

    pub fn process_empty_references(&mut self) {
        if self.references.is_empty() {
            self.references = self
                .defines
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().cloned().collect::<Vec<PathBuf>>()))
                .collect();
        }
    }

    pub fn process_common_tags(&mut self) {
        self.common_tags = self
            .defines
            .keys()
            .filter_map(|key| match self.references.contains_key(key) {
                true => Some(key.clone()),
                false => None,
            })
            .collect();
    }

    pub fn debug_print(&self) {
        println!("==========Defines==========");
        self.defines.iter().for_each(|(key, set)| {
            println!("Key {}, Set: {:?}", key, set);
        });

        println!("==========Definitions==========");
        self.definitions
            .iter()
            .for_each(|((pathbuf, tag_name), set)| {
                println!("Key {:?}, Set: {:?}", (pathbuf, tag_name), set);
            });

        println!("==========References==========");
        self.references.iter().for_each(|(tag_name, paths)| {
            println!("Tag: {}, Paths: {:?}", tag_name, paths);
        });

        println!("==========Common Tags==========");
        self.common_tags.iter().for_each(|tag| {
            println!(
                "Common Tag: {}\n(defined in: {:?}, referenced in: {:?})",
                tag, &self.defines[tag], &self.references[tag]
            );
        });
    }

    // Add methods to query the index as needed
}
