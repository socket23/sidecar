use crate::repo::types::{RepoRef, SyncStatus};

#[derive(serde::Serialize, Clone)]
pub struct Progress {
    #[serde(rename = "ref")]
    reporef: RepoRef,
    #[serde(rename = "ev")]
    event: ProgressEvent,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ProgressEvent {
    IndexPercent(u8),
    StatusChange(SyncStatus),
}
