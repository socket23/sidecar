// This is where we will create the default webserver for running the binary
// locally

use axum::routing::get;
use axum::Extension;
use clap::Parser;
use sidecar::application::{application::Application, config::configuration::Configuration};
use std::net::SocketAddr;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer};

pub type Router<S = Application> = axum::Router<S>;

#[tokio::main]
async fn main() {
    let configuration = Configuration::parse();
    // We get the logging setup first
    Application::install_logging(&configuration);

    // We initialize the logging here
    let application = Application::initialize(configuration).await;
    let _ = start(application).await;
    println!("Hello world! application");
}

pub async fn start(app: Application) -> anyhow::Result<()> {
    let bind = SocketAddr::new(app.config.host.parse()?, app.config.port);
    let api = Router::new().route("/config", get(sidecar::webserver::config::get));

    let api = api
        .layer(Extension(app.clone()))
        .with_state(app.clone())
        .layer(CorsLayer::permissive())
        .layer(CatchPanicLayer::new());

    let router = Router::new().nest("/api", api);

    axum::Server::bind(&bind)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
