// This is the place where we handle all the routes with respect to the repos
// and how we are going to index them.

use std::{collections::HashMap, time::Duration};

use axum::{
    extract::{Query, State},
    response::{sse, IntoResponse, Sse},
    Extension,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::application::Application,
    repo::types::{Backend, RepoRef, Repository, SyncStatus},
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

#[derive(Deserialize, Serialize)]
pub struct RepoStatus {
    pub repo_map: HashMap<RepoRef, Repository>,
}

impl ApiResponse for RepoStatus {}

/// Synchronize a repo by its id
pub async fn sync(
    Query(RepoParams { repo }): Query<RepoParams>,
    State(app): State<Application>,
) -> Result<impl IntoResponse> {
    // TODO: We can refactor `repo_pool` to also hold queued repos, instead of doing a calculation
    // like this which is prone to timing issues.
    let _ = app.repo_pool.len();
    let _ = app.write_index().enqueue_sync(vec![repo]).await;

    Ok(json(ReposResponse::SyncQueued))
}

/// Get a stream of status notifications about the indexing of each repository
/// This endpoint opens an SSE stream
//
pub async fn index_status(Extension(app): Extension<Application>) -> impl IntoResponse {
    let mut receiver = app.sync_queue.subscribe();
    let progress_context = app.sync_queue.get_progress_context().to_owned();

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
            .event(sse::Event::default().event(format!("keep_alive {}", progress_context))),
    )
}

/// Get the status of the queue which we are processing
pub async fn queue_status(State(app): State<Application>) -> impl IntoResponse {
    json(ReposResponse::SyncQueue(app.sync_queue.read_queue().await))
}

// Get the status of the various repositories
pub async fn repo_status(State(app): State<Application>) -> impl IntoResponse {
    let mut repo_map: HashMap<_, _> = Default::default();
    app.repo_pool
        .scan_async(|repo_name, state| {
            repo_map.insert(repo_name.clone(), state.clone());
        })
        .await;
    json(RepoStatus { repo_map })
}
