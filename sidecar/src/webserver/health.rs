use axum::Extension;

use crate::application::application::Application;

/// We send a HC check over here

pub async fn health(Extension(_app): Extension<Application>) {}
