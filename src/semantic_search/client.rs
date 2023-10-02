/// This contains the main client we will be using for semantic search
/// The client provides additional support for querying using qdrant and exposes
/// the embedder which we want to use
use std::{env, path::Path, sync::Arc};

use qdrant_client::{client::QdrantClient, prelude::QdrantClientConfig};

use crate::{
    application::config::configuration::Configuration,
    embedder::embedder::{Embedder, LocalEmbedder},
};

pub struct SemanticClient {
    embedder: Arc<dyn Embedder>,
    search_client: Arc<QdrantClient>,
    config: Arc<Configuration>,
}

impl SemanticClient {
    pub fn new(config: Arc<Configuration>) -> Option<Self> {
        if config.qdrant_url.is_none() {
            return None;
        }
        let qdrant_config = config
            .qdrant_url
            .as_ref()
            .map(|url| QdrantClientConfig::from_url(&url));
        let qdrant_client = QdrantClient::new(qdrant_config);

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
        match qdrant_client {
            Ok(client) => Some(Self {
                embedder: Arc::new(embedder.expect("is_err check above")),
                search_client: Arc::new(client),
                config,
            }),
            Err(_) => None,
        }
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
