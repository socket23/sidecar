/// This contains the main client we will be using for semantic search
/// The client provides additional support for querying using qdrant and exposes
/// the embedder which we want to use
use std::{env, path::Path, sync::Arc};

use qdrant_client::{
    client::QdrantClient,
    prelude::QdrantClientConfig,
    qdrant::{
        vectors_config, CollectionOperationResponse, CreateCollection, Distance, VectorParams,
        VectorsConfig,
    },
};
use tracing::debug;

use crate::{
    application::config::configuration::Configuration,
    embedder::embedder::{Embedder, LocalEmbedder},
};

const EMBEDDING_DIM: usize = 384;

pub struct SemanticClient {
    embedder: Arc<dyn Embedder>,
    search_client: Arc<QdrantClient>,
    config: Arc<Configuration>,
}

impl SemanticClient {
    pub async fn new(config: Arc<Configuration>) -> Option<Self> {
        if config.qdrant_url.is_none() {
            return None;
        }
        let qdrant_config = config
            .qdrant_url
            .as_ref()
            .map(|url| QdrantClientConfig::from_url(&url));
        let qdrant_client = QdrantClient::new(qdrant_config).expect("client creation to not fail");

        match qdrant_client.has_collection(&config.collection_name).await {
            Ok(false) => {
                let CollectionOperationResponse { result, time } =
                    create_collection(&config.collection_name, &qdrant_client)
                        .await
                        .unwrap();

                debug!(
                    "Created collection {} in {}ms with result {}",
                    config.collection_name, time, result
                );

                assert!(result);
            }
            Ok(true) => {}
            Err(_) => return None,
        }

        // TODO(skcd): we might want to create some indexes here, but we can
        // figure that out as we keep hacking

        let dylib_directory = config.dylib_directory.as_ref();

        if dylib_directory.is_none() {
            return None;
        }

        // Now we first need to set the ort library up properly
        init_ort_dylib(dylib_directory.expect("is_none check above"));
        let embedder = LocalEmbedder::new(&config.model_dir);
        if embedder.is_err() {
            return None;
        }
        Some(Self {
            embedder: Arc::new(embedder.expect("is_err check above")),
            search_client: Arc::new(qdrant_client),
            config,
        })
    }
}

/// Initialize the `ORT_DYLIB_PATH` variable, consumed by the `ort` crate.
///
/// This is required because we need the dylib library to be present when we are
/// starting out the embedder as this is required by the ort runtime.
fn init_ort_dylib(dylib_dir: impl AsRef<Path>) {
    {
        #[cfg(target_os = "linux")]
        let lib_name = "libonnxruntime.so";
        #[cfg(target_os = "macos")]
        let lib_name = "libonnxruntime.dylib";
        #[cfg(target_os = "windows")]
        let lib_name = "onnxruntime.dll";

        let ort_dylib_path = dylib_dir.as_ref().join(lib_name);

        if env::var("ORT_DYLIB_PATH").is_err() {
            env::set_var("ORT_DYLIB_PATH", ort_dylib_path);
        }
    }
}

pub(super) async fn create_collection(
    name: &str,
    qdrant: &QdrantClient,
) -> anyhow::Result<CollectionOperationResponse> {
    qdrant
        .create_collection(&CreateCollection {
            collection_name: name.to_string(),
            vectors_config: Some(VectorsConfig {
                config: Some(vectors_config::Config::Params(VectorParams {
                    size: EMBEDDING_DIM as u64,
                    distance: Distance::Cosine.into(),
                    on_disk: Some(true),
                    ..Default::default()
                })),
            }),
            ..Default::default()
        })
        .await
}
