// This is the place where we handle all the routes with respect to the repos
// and how we are going to index them.

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::application::Application,
    repo::types::{Backend, RepoRef, SyncStatus},
};

use super::types::{json, ApiResponse, Result};

#[derive(Serialize, Debug, Eq)]
pub(crate) struct Repo {
    pub(super) provider: Backend,
    pub(super) name: String,
    #[serde(rename = "ref")]
    pub(super) repo_ref: RepoRef,
    pub(super) local_duplicates: Vec<RepoRef>,
    pub(super) sync_status: SyncStatus,
    pub(super) most_common_lang: Option<String>,
}

impl PartialEq for Repo {
    fn eq(&self, other: &Self) -> bool {
        self.repo_ref == other.repo_ref
    }
}

#[derive(serde::Serialize, Debug)]
pub struct QueuedRepoStatus {
    pub reporef: RepoRef,
    pub state: QueueState,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueueState {
    Active,
    Queued,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReposResponse {
    List(Vec<Repo>),
    Item(Repo),
    SyncQueue(Vec<QueuedRepoStatus>),
    SyncQueued,
    Deleted,
}

#[derive(Deserialize)]
pub(crate) struct RepoParams {
    pub(crate) repo: RepoRef,
}

impl ApiResponse for ReposResponse {}

/// Synchronize a repo by its id
pub(super) async fn sync(
    Query(RepoParams { repo }): Query<RepoParams>,
    State(app): State<Application>,
) -> Result<impl IntoResponse> {
    // TODO: We can refactor `repo_pool` to also hold queued repos, instead of doing a calculation
    // like this which is prone to timing issues.
    let num_repos = app.repo_pool.len();
    let num_queued = app.write_index().enqueue_sync(vec![repo]).await;

    Ok(json(ReposResponse::SyncQueued))
}
