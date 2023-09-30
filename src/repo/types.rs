use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct RepoMetadata {
    // keep track of the last commit timestamp here and nothing else for now
    pub last_commit_unix_secs: Option<i64>,
}

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

    pub fn is_local(&self) -> bool {
        matches!(self.backend, Backend::Local)
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

    /// This was removed from the remote url, so yolo
    RemoteRemoved,
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
    pub last_commit_unix_secs: i64,
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

    pub(crate) fn sync_done_with(&mut self, metadata: Arc<RepoMetadata>) {
        self.last_index_unix_secs = get_unix_time(SystemTime::now());
        self.last_commit_unix_secs = metadata.last_commit_unix_secs.unwrap_or(0);
        self.most_common_lang = Some("not_set".to_owned());

        self.sync_status = SyncStatus::Done;
    }

    /// Pre-scan the repository to provide supporting metadata for a
    /// new indexing operation
    pub async fn get_repo_metadata(&self) -> Arc<RepoMetadata> {
        let last_commit_unix_secs = gix::open(&self.disk_path)
            .context("failed to open git repo")
            .and_then(|repo| Ok(repo.head()?.peel_to_commit_in_place()?.time()?.seconds))
            .ok();

        RepoMetadata {
            last_commit_unix_secs,
        }
        .into()
    }
}

fn get_unix_time(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time error")
        .as_secs()
}
