// This is the place where we handle all the routes with respect to the repos
// and how we are going to index them.

use std::time::Duration;

use axum::{
    extract::{Query, State},
    response::{sse, IntoResponse, Sse},
    Extension,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::application::Application,
    repo::types::{Backend, RepoRef, SyncStatus},
};

use super::types::{json, ApiResponse, Result};

#[derive(Serialize, Debug, Eq)]
pub struct Repo {
    pub provider: Backend,
    pub name: String,
    #[serde(rename = "ref")]
    pub repo_ref: RepoRef,
    pub local_duplicates: Vec<RepoRef>,
    pub sync_status: SyncStatus,
    pub most_common_lang: Option<String>,
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
pub enum ReposResponse {
    List(Vec<Repo>),
    Item(Repo),
    SyncQueue(Vec<QueuedRepoStatus>),
    SyncQueued,
    Deleted,
}

#[derive(Deserialize, Serialize)]
pub struct RepoParams {
    pub repo: RepoRef,
}

impl ApiResponse for ReposResponse {}

/// Synchronize a repo by its id
pub async fn sync(
    Query(repo_ref): Query<RepoRef>,
    State(app): State<Application>,
) -> Result<impl IntoResponse> {
    // TODO: We can refactor `repo_pool` to also hold queued repos, instead of doing a calculation
    // like this which is prone to timing issues.
    let num_repos = app.repo_pool.len();
    let num_queued = app.write_index().enqueue_sync(vec![repo_ref]).await;

    Ok(json(ReposResponse::SyncQueued))
}

/// Get a stream of status notifications about the indexing of each repository
/// This endpoint opens an SSE stream
//
pub async fn index_status(Extension(app): Extension<Application>) -> impl IntoResponse {
    let mut receiver = app.sync_queue.subscribe();

    Sse::new(async_stream::stream! {
        while let Ok(event) = receiver.recv().await {
            yield sse::Event::default().json_data(event).map_err(|err| {
                <_ as Into<Box<dyn std::error::Error + Send + Sync>>>::into(err)
            });
        }
    })
    .keep_alive(
        sse::KeepAlive::new()
            .interval(Duration::from_secs(2))
            .event(sse::Event::default().event("heartbeat")),
    )
}
