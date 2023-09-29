use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// Types of repo
#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Local,
    // Github, (We don't support this yet)
}

// Repository identifier
#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct RepoRef {
    pub backend: Backend,
    pub name: String,
}

impl RepoRef {
    pub fn local_path(&self) -> Option<PathBuf> {
        match self.backend {
            Backend::Local => Some(PathBuf::from(&self.name)),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<P: AsRef<Path>> From<&P> for RepoRef {
    fn from(path: &P) -> Self {
        RepoRef {
            backend: Backend::Local,
            name: path.as_ref().to_string_lossy().to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// There was an error during last sync & index
    Error { message: String },

    /// Repository is not yet managed by bloop
    Uninitialized,

    /// The user requested cancelling the process
    Cancelling,

    /// Last sync & index cancelled by the user
    Cancelled,

    /// Queued for sync & index
    Queued,

    /// Active VCS operation in progress
    Syncing,

    /// Active indexing in progress
    Indexing,

    /// Successfully indexed
    Done,

    /// Removed from the index
    Removed,
}

impl SyncStatus {
    pub fn indexable(&self) -> bool {
        matches!(self, Self::Done | Self::Queued | Self::Error { .. })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Repository {
    pub disk_path: PathBuf,
    pub sync_status: SyncStatus,
    pub last_commit_unix_secs: u64,
    pub last_index_unix_secs: u64,
    pub most_common_lang: Option<String>,
}

impl Repository {
    /// Marks the repository for removal on the next sync
    /// Does not initiate a new sync.
    pub(crate) fn mark_removed(&mut self) {
        self.sync_status = SyncStatus::Removed;
    }

    /// Marks the repository for indexing on the next sync
    /// Does not initiate a new sync.
    pub(crate) fn mark_queued(&mut self) {
        self.sync_status = SyncStatus::Queued;
    }

    pub(crate) fn local_from(repo_ref: &RepoRef) -> Self {
        let disk_path = repo_ref.local_path().unwrap();

        // TODO(codestory): Add the last commit timestamp here because we are passing
        // 0 right now :|
        Self {
            sync_status: SyncStatus::Queued,
            last_index_unix_secs: 0,
            last_commit_unix_secs: 0,
            disk_path,
            most_common_lang: None,
        }
    }
}
