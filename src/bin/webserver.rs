// This is where we will create the default webserver for running the binary
// locally

use anyhow::Result;
use axum::routing::get;
use axum::Extension;
use clap::Parser;
use sidecar::{
    application::{application::Application, config::configuration::Configuration},
    bg_poll::background_polling::poll_repo_updates,
    semantic_search::qdrant_process::{wait_for_qdrant, QdrantServerProcess},
};
use std::net::SocketAddr;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer};
use tracing::{debug, error, info};

pub type Router<S = Application> = axum::Router<S>;

#[tokio::main]
async fn main() -> Result<()> {
    info!("CodeStory 🚀");
    let configuration = Configuration::parse();

    // Star the qdrant server and make sure that it has started up
    let _qdrant_process = QdrantServerProcess::initialize(&configuration).await?;
    // HC the process here to make sure that it has started up
    wait_for_qdrant().await;
    debug!("qdrant server started");

    // We get the logging setup first
    debug!("installing logging to local file");
    Application::install_logging(&configuration);

    // We initialize the logging here
    let application = Application::initialize(configuration).await?;
    debug!("initialized application");

    // Start the webserver
    let _ = run(application).await;
    Ok(())
}

pub async fn run(application: Application) -> Result<()> {
    let mut joins = tokio::task::JoinSet::new();

    // Start background tasks here
    tokio::spawn(poll_repo_updates(application.clone()));

    joins.spawn(start(application));

    while let Some(result) = joins.join_next().await {
        if let Ok(Err(err)) = result {
            error!(?err, "sidecar failed");
            return Err(err);
        }
    }

    Ok(())
}

// TODO(skcd): Add routes here which can do the following:
// - when a file changes, it should still be logged and tracked
// - when a file is opened, it should be tracked over here too
pub async fn start(app: Application) -> anyhow::Result<()> {
    let bind = SocketAddr::new(app.config.host.parse()?, app.config.port);
    let api = Router::new()
        .route("/config", get(sidecar::webserver::config::get))
        .route(
            "/reach_the_devs",
            get(sidecar::webserver::config::reach_the_devs),
        )
        .nest("/repo", repo_router());

    let api = api
        .layer(Extension(app.clone()))
        .with_state(app.clone())
        .with_state(app.clone())
        .layer(CorsLayer::permissive())
        .layer(CatchPanicLayer::new());

    let router = Router::new().nest("/api", api);

    axum::Server::bind(&bind)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

fn repo_router() -> Router {
    use axum::routing::*;
    Router::new()
        // 127.0.0.1:42424/api/repo/sync?backend=local/{path_absolute}
        .route("/sync", get(sidecar::webserver::repos::sync))
        .route("/status", get(sidecar::webserver::repos::index_status))
        // Gives back the status of the queue
        .route("/queue", get(sidecar::webserver::repos::queue_status))
        // Gives back the repos we know about
        .route("/repo_list", get(sidecar::webserver::repos::repo_status))
}

// TODO(skcd): Now we have to do the following: start the qdrant binary
// and also setup the client properly so we can use qdrant, we can also not do
// this and keep going with raw embedding search and make it happen, but this
// will be better in the long term (and also its not too difficult)
