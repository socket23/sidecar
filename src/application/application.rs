// This is where we will define the core application and all the related things
// on how to startup the application

use std::sync::Arc;

use once_cell::sync::OnceCell;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    fmt,
    prelude::*,
};

use crate::repo::state::RepositoryPool;

use super::{config::configuration::Configuration, logging::tracing::tracing_subscribe};

static LOGGER_INSTALLED: OnceCell<bool> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct Application {
    pub config: Configuration,
    pub repo_pool: RepositoryPool,
    // pub indexes: Arc<Indexes>,
}

impl Application {
    pub async fn initialize(mut config: Configuration) -> anyhow::Result<Self> {
        config.max_threads = config.max_threads.max(minimum_parallelism());
        config.state_source.set_default_dir(&config.index_dir);
        let repo_pool = config.state_source.initialize_pool()?;
        Ok(Self { config, repo_pool })
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

// We need at the very least 1 thread to do background work
fn minimum_parallelism() -> usize {
    1
}
