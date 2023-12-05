// This is where we will define the core application and all the related things
// on how to startup the application

use std::sync::Arc;

use once_cell::sync::OnceCell;
use tracing::{debug, warn};

use crate::{
    chunking::languages::TSLanguageParsing,
    db::sqlite::{self, SqlDb},
    llm::types::LLMCustomConfig,
    reporting::posthog::client::{posthog_client, PosthogClient},
    semantic_search::client::SemanticClient,
};
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
    pub semantic_client: Option<SemanticClient>,
    /// Background & maintenance tasks are executed on a separate
    /// executor
    pub sync_queue: SyncQueue,
    /// We also want to keep the language parsing functionality here
    pub language_parsing: Arc<TSLanguageParsing>,
    pub sql: SqlDb,
    pub posthog_client: Arc<PosthogClient>,
    pub user_id: String,
    pub llm_config: LLMCustomConfig,
}

impl Application {
    pub async fn initialize(mut config: Configuration) -> anyhow::Result<Self> {
        config.max_threads = config.max_threads.max(minimum_parallelism());
        // Setting the directory for the state and where we will be storing
        // things
        config.state_source.set_default_dir(&config.index_dir);
        debug!(?config, "configuration after loading");
        let repo_pool = config.state_source.initialize_pool()?;
        let config = Arc::new(config);
        let llm_config = if let Some(llm_endpoint) = config.llm_endpoint.as_ref() {
            LLMCustomConfig::mistral(llm_endpoint.to_owned())
        } else {
            LLMCustomConfig::openai()
        };
        let sync_queue = SyncQueue::start(config.clone());
        let sql_db = Arc::new(sqlite::init(config.clone()).await?);
        let language_parsing = Arc::new(TSLanguageParsing::init());
        let semantic_client = SemanticClient::new(config.clone(), language_parsing.clone()).await;
        let posthog_client = posthog_client(&config.user_id);
        debug!("semantic client presence: {}", semantic_client.is_some());
        Ok(Self {
            config: config.clone(),
            repo_pool: repo_pool.clone(),
            semantic_client: semantic_client.clone(),
            indexes: Indexes::new(
                repo_pool,
                sql_db.clone(),
                semantic_client,
                config.clone(),
                language_parsing.clone(),
            )
            .await?
            .into(),
            sync_queue,
            language_parsing,
            sql: sql_db,
            posthog_client: Arc::new(posthog_client),
            user_id: config.user_id.clone(),
            llm_config,
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
