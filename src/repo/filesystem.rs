/*
We are going to look at the file system and iterate along all the files
which might be present here
*/

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::{
    application::background::SyncPipes,
    repo::iterator::{RepositoryDirectory, RepositoryFile},
};

use super::iterator::{should_index_entry, FileSource, RepoDirectoryEntry};

pub const AVG_LINE_LEN: u64 = 30;
pub const MAX_LINE_COUNT: u64 = 20000;
pub const MAX_FILE_LEN: u64 = AVG_LINE_LEN * MAX_LINE_COUNT;

pub struct FileWalker {
    pub file_list: Vec<PathBuf>,
}

impl FileWalker {
    pub fn index_directory(dir: impl AsRef<Path>) -> FileWalker {
        // note: this WILL observe .gitignore files for the respective repos.
        let walker = WalkBuilder::new(&dir)
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

impl FileSource for FileWalker {
    fn len(&self) -> usize {
        self.file_list.len()
    }

    fn for_each(self, signal: &SyncPipes, iterator: impl Fn(RepoDirectoryEntry) + Sync + Send) {
        use rayon::prelude::*;
        // using the rayon parallel iterator here so we can walk the directory
        // in parallel
        self.file_list
            .into_par_iter()
            .filter_map(|entry_disk_path| {
                if entry_disk_path.is_file() {
                    let buffer = match std::fs::read_to_string(&entry_disk_path) {
                        Err(_) => {
                            return None;
                        }
                        Ok(buffer) => buffer,
                    };
                    Some(RepoDirectoryEntry::File(RepositoryFile {
                        buffer,
                        path: entry_disk_path.to_string_lossy().to_string(),
                    }))
                } else if entry_disk_path.is_dir() {
                    Some(RepoDirectoryEntry::Dir(RepositoryDirectory {
                        path: entry_disk_path.to_string_lossy().to_string(),
                    }))
                } else {
                    Some(RepoDirectoryEntry::Other)
                }
            })
            .take_any_while(|_| !signal.is_cancelled())
            .for_each(iterator);
    }
}
