/// This contains the main client we will be using for semantic search
/// The client provides additional support for querying using qdrant and exposes
/// the embedder which we want to use
use std::sync::Arc;

use qdrant_client::{client::QdrantClient, prelude::QdrantClientConfig};

use crate::{application::config::configuration::Configuration, embedder::embedder::Embedder};

pub struct SemanticClient {
    embedder: Arc<dyn Embedder>,
    search_client: Arc<QdrantClient>,
    config: Arc<Configuration>,
}

impl SemanticClient {
    pub fn new(embedder: Arc<dyn Embedder>, config: Arc<Configuration>) -> Option<Self> {
        if config.qdrant_url.is_none() {
            return None;
        }
        let qdrant_config = config
            .qdrant_url
            .as_ref()
            .map(|url| QdrantClientConfig::from_url(&url));
        let qdrant_client = QdrantClient::new(qdrant_config);
        match qdrant_client {
            Ok(client) => Some(Self {
                embedder,
                search_client: Arc::new(client),
                config,
            }),
            Err(_) => None,
        }
    }
}
