/// This contains the main client we will be using for semantic search
/// The client provides additional support for querying using qdrant and exposes
/// the embedder which we want to use
use std::sync::Arc;

use qdrant_client::client::QdrantClient;

use crate::{application::config::configuration::Configuration, embedder::embedder::Embedder};

pub struct SemanticClient {
    embedder: Arc<dyn Embedder>,
    search_client: Arc<QdrantClient>,
    config: Arc<Configuration>,
}

impl SemanticClient {
    pub fn new(
        embedder: Arc<dyn Embedder>,
        search_client: Arc<QdrantClient>,
        config: Arc<Configuration>,
    ) -> Self {
        Self {
            embedder,
            search_client,
            config,
        }
    }
}
