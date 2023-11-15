// This is where we handle the config get operation so we can look at what
// the config is

use axum::{extract::State, response::IntoResponse};
use serde::Serialize;

use crate::application::application::Application;
use crate::state::BINARY_VERSION_HASH;

use super::types::json;
use super::types::ApiResponse;

#[derive(Serialize, Debug)]
pub(super) struct ConfigResponse {
    response: String,
    model_dir: String,
}

#[derive(Serialize, Debug)]
pub(super) struct ReachTheDevsResponse {
    response: String,
}

#[derive(Serialize, Debug)]
pub(super) struct VersionResponse {
    version_hash: String,
}

impl ApiResponse for ConfigResponse {}

impl ApiResponse for ReachTheDevsResponse {}

impl ApiResponse for VersionResponse {}

pub async fn get(State(app): State<Application>) -> impl IntoResponse {
    json(ConfigResponse {
        response: "hello_skcd".to_owned(),
        model_dir: app.config.model_dir.to_str().unwrap().to_owned(),
    })
}

pub async fn version(State(_): State<Application>) -> impl IntoResponse {
    json(VersionResponse {
        version_hash: BINARY_VERSION_HASH.to_owned(),
    })
}

pub async fn reach_the_devs() -> impl IntoResponse {
    json(ReachTheDevsResponse {
        response: r#"
        You made it here! Reach out to skcd@codestory.ai or ghost@codestory.ai, we would love to talk to you about joining codestory's hacker first dev team
        "#.to_owned(),
    })
}
