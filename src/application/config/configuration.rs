use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use clap::Parser;
use gix::config::boolean;
use serde::{Deserialize, Serialize};

use crate::repo::state::StateSource;

#[derive(Serialize, Deserialize, Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Configuration {
    #[clap(short, long, default_value_os_t = default_index_dir())]
    #[serde(default = "default_index_dir")]
    /// Directory to store all persistent state
    pub index_dir: PathBuf,

    #[clap(long, default_value_t = default_port())]
    #[serde(default = "default_port")]
    /// Bind the webserver to `<host>`
    pub port: u16,

    #[clap(long)]
    /// Path to the embedding model directory
    pub model_dir: PathBuf,

    #[clap(long, default_value_t = default_host())]
    #[serde(default = "default_host")]
    /// Bind the webserver to `<port>`
    pub host: String,

    #[clap(flatten)]
    #[serde(default)]
    pub state_source: StateSource,

    #[clap(short, long, default_value_t = default_parallelism())]
    #[serde(default = "default_parallelism")]
    /// Maximum number of parallel background threads
    pub max_threads: usize,

    #[clap(short, long, default_value_t = default_buffer_size())]
    #[serde(default = "default_buffer_size")]
    /// Size of memory to use for file indexes
    pub buffer_size: usize,

    /// Qdrant url here can be mentioned if we are running it remotely or have
    /// it running on its own process
    #[clap(long)]
    #[serde(default = "default_qdrant_url")]
    pub qdrant_url: String,

    /// The folder where the qdrant binary is present so we can start the server
    /// and power the qdrant client
    #[clap(short, long)]
    pub qdrant_binary_directory: Option<PathBuf>,

    /// The location for the dylib directory where we have the runtime binaries
    /// required for ort
    #[clap(short, long)]
    pub dylib_directory: PathBuf,

    /// Qdrant allows us to create collections and we need to provide it a default
    /// value to start with
    #[clap(short, long, default_value_t = default_collection_name())]
    #[serde(default = "default_collection_name")]
    pub collection_name: String,

    #[clap(long, default_value_t = interactive_batch_size())]
    #[serde(default = "interactive_batch_size")]
    /// Batch size for batched embeddings
    pub embedding_batch_len: NonZeroUsize,

    #[clap(long, default_value_t = default_user_id())]
    #[serde(default = "default_user_id")]
    user_id: String,

    /// If we should poll the local repo for updates auto-magically. Disabled
    /// by default, until we figure out the delta sync method where we only
    /// reindex the files which have changed
    #[clap(long)]
    pub enable_background_polling: bool,
}

impl Configuration {
    /// Directory where logs are written to
    pub fn log_dir(&self) -> PathBuf {
        self.index_dir.join("logs")
    }

    pub fn index_path(&self, name: impl AsRef<Path>) -> impl AsRef<Path> {
        self.index_dir.join(name)
    }

    pub fn qdrant_storage(&self) -> PathBuf {
        self.index_dir.join("qdrant_storage")
    }
}

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

fn default_port() -> u16 {
    42424
}

fn default_host() -> String {
    "127.0.0.1".to_owned()
}

pub fn default_parallelism() -> usize {
    std::thread::available_parallelism().unwrap().get()
}

const fn default_buffer_size() -> usize {
    100_000_000
}

fn default_collection_name() -> String {
    "codestory".to_owned()
}

fn interactive_batch_size() -> NonZeroUsize {
    NonZeroUsize::new(1).unwrap()
}

fn default_qdrant_url() -> String {
    "http://127.0.0.1:6334".to_owned()
}

fn default_user_id() -> String {
    "codestory".to_owned()
}
