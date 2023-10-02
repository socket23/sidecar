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

use crate::db::sqlite::{self, SqlDb};
use crate::{indexes::indexer::Indexes, repo::state::RepositoryPool};

use super::{
    background::{BoundSyncQueue, SyncQueue},
    config::configuration::Configuration,
    logging::tracing::tracing_subscribe,
};

static LOGGER_INSTALLED: OnceCell<bool> = OnceCell::new();

#[derive(Clone)]
pub struct Application {
    // Arc here because its shared by many things and is the consistent state
    // for the application
    pub config: Arc<Configuration>,
    pub repo_pool: RepositoryPool,
    pub indexes: Arc<Indexes>,
    /// Background & maintenance tasks are executed on a separate
    /// executor
    pub sync_queue: SyncQueue,
    pub sql: SqlDb,
}

impl Application {
    pub async fn initialize(mut config: Configuration) -> anyhow::Result<Self> {
        config.max_threads = config.max_threads.max(minimum_parallelism());
        config.state_source.set_default_dir(&config.index_dir);
        debug!(?config, "effective configuration");
        let repo_pool = config.state_source.initialize_pool()?;
        let config = Arc::new(config);
        let sync_queue = SyncQueue::start(config.clone());
        let sql_db = Arc::new(sqlite::init(config.clone()).await?);
        Ok(Self {
            config: config.clone(),
            repo_pool: repo_pool.clone(),
            indexes: Indexes::new(repo_pool, sql_db.clone(), config)
                .await?
                .into(),
            sync_queue,
            sql: sql_db,
        })
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

    pub fn write_index(&self) -> BoundSyncQueue {
        self.sync_queue.bind(self.clone())
    }
}

// We need at the very least 1 thread to do background work
fn minimum_parallelism() -> usize {
    1
}
