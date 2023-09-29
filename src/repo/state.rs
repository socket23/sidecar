use clap::Args;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use super::types::{RepoRef, Repository};

pub type RepositoryPool = Arc<scc::HashMap<RepoRef, Repository>>;

#[derive(Serialize, Deserialize, Args, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct StateSource {
    #[serde(default)]
    directory: Option<PathBuf>,
    // state file where we store the status of each repository
    #[serde(default)]
    repo_state_file: Option<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum RepoError {
    #[error("local repository must have an absolute path")]
    NonAbsoluteLocal,
    #[error("paths can't contain `..` or `.`")]
    InvalidPath,
    #[error("indexing error")]
    Anyhow {
        #[from]
        error: anyhow::Error,
    },
}

impl StateSource {
    pub fn set_default_dir(&mut self, dir: &Path) {
        std::fs::create_dir_all(dir).expect("the index folder can't be created");

        self.repo_state_file
            .get_or_insert_with(|| dir.join("repo_state"));

        self.directory.get_or_insert_with(|| {
            let target = dir.join("local_cache");
            std::fs::create_dir_all(&target).unwrap();

            target
        });
    }
}
