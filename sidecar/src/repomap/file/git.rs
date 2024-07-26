use std::{collections::HashMap, path::Path};

use super::errors::FileError;

#[derive(Hash, Eq, PartialEq)]
enum FileType {
    File,
    Directory,
    NotTracked,
}

pub struct GitWalker {}

impl GitWalker {
    pub fn read_files(&self, directory: &Path) -> Result<HashMap<String, Vec<u8>>, FileError> {
        let git = gix::open::Options::isolated()
            .filter_config_section(|_| false)
            .open(directory);

        if let Err(_) = git {
            // load from local fs using recursive calls
            let mut files = HashMap::new();
            println!(
                "git_walker::reading_files_locally::without_git({:?})",
                &directory
            );
            let _ = self.get_files_recursive(directory, &mut files);
            return Ok(files);
        }

        let git = git.expect("if let Err to hold");

        let local_git = git.to_thread_local();
        let mut head = local_git.head().map_err(|_e| FileError::GixError)?;
        let trees = vec![(
            true,
            "HEAD".to_owned(),
            head.peel_to_commit_in_place()
                .map_err(|_e| FileError::GixError)?
                .tree()
                .map_err(|_e| FileError::GixError)?,
        )];

        let directory_ref: &Path = directory.as_ref();

        Ok(trees
            .into_iter()
            .flat_map(|(is_head, branch, tree)| {
                let files = tree.traverse().breadthfirst.files().unwrap().into_iter();

                files.map(move |entry| {
                    let strpath = String::from_utf8_lossy(entry.filepath.as_ref());
                    let full_path = directory_ref.join(strpath.as_ref());
                    (
                        is_head,
                        branch.clone(),
                        full_path.to_string_lossy().to_string(),
                        entry.mode,
                        entry.oid,
                    )
                })
            })
            .filter_map(|(_, _, file, mode, oid)| {
                let kind = if mode.is_tree() {
                    FileType::Directory
                } else if mode.is_blob() {
                    FileType::File
                } else {
                    FileType::NotTracked
                };

                let git = git.to_thread_local();
                let Ok(Some(object)) = git.try_find_object(oid) else {
                    return None;
                };

                match kind {
                    FileType::File => Some((file, object.data.to_vec())),
                    _ => None,
                }
            })
            .collect::<HashMap<String, Vec<u8>>>())
    }

    fn get_files_recursive(
        &self,
        dir: &Path,
        files: &mut HashMap<String, Vec<u8>>,
    ) -> Result<(), FileError> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let file_conetent = std::fs::read(path.to_owned())?;
                // Add the file content over here
                files.insert(
                    path.to_str().expect("to not fail").to_owned(),
                    file_conetent,
                );
            } else if path.is_dir() {
                self.get_files_recursive(&path, files)?;
            }
        }
        Ok(())
    }
}
