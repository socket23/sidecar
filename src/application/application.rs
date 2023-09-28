// This is where we will define the core application and all the related things
// on how to startup the application

use once_cell::sync::OnceCell;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    fmt,
    prelude::*,
};

use super::{config::configuration::Configuration, logging::tracing::tracing_subscribe};

static LOGGER_INSTALLED: OnceCell<bool> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct Application {
    pub config: Configuration,
}

impl Application {
    pub async fn initialize(config: Configuration) -> Self {
        Self { config }
    }

    pub fn install_logging(config: &Configuration) {
        if let Some(true) = LOGGER_INSTALLED.get() {
            return;
        }

        if !tracing_subscribe(config) {
            warn!("Failed to install tracing_subscriber. There's probably one already...");
        };

        if color_eyre::install().is_err() {
            warn!("Failed to install color-eyre. Oh well...");
        };

        LOGGER_INSTALLED.set(true).unwrap();
    }
}
