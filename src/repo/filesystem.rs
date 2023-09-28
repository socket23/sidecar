/*
We are going to look at the file system and iterate along all the files
which might be present here
*/

use std::path::{Path, PathBuf};

use super::iterator::should_index_entry;

pub const AVG_LINE_LEN: u64 = 30;
pub const MAX_LINE_COUNT: u64 = 20000;
pub const MAX_FILE_LEN: u64 = AVG_LINE_LEN * MAX_LINE_COUNT;

pub struct FileWalker {
    pub file_list: Vec<PathBuf>,
}

impl FileWalker {
    pub fn index_directory(dir: impl AsRef<Path>) -> FileWalker {
        // note: this WILL observe .gitignore files for the respective repos.
        let walker = ignore::WalkBuilder::new(&dir)
            .standard_filters(true)
            .hidden(false)
            .filter_entry(should_index_entry)
            .build();

        let file_list = walker
            .filter_map(|de| match de {
                Ok(de) => Some(de),
                Err(_) => None,
            })
            // Preliminarily ignore files that are very large, without reading the contents.
            .filter(|de| matches!(de.metadata(), Ok(meta) if meta.len() < MAX_FILE_LEN))
            .filter_map(|de| std::fs::canonicalize(de.into_path()).ok())
            .collect();

        Self { file_list }
    }
}
