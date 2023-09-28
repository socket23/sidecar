// This is where we handle the config get operation so we can look at what
// the config is

use axum::{extract::State, response::IntoResponse};
use serde::Serialize;

use crate::application::application::Application;

use super::types::json;
use super::types::ApiResponse;

#[derive(Serialize, Debug)]
pub(super) struct ConfigResponse {
    response: String,
}

impl ApiResponse for ConfigResponse {}

pub async fn get(State(app): State<Application>) -> impl IntoResponse {
    json(ConfigResponse {
        response: "hello skcd".to_owned(),
    })
}
