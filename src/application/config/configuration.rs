use std::path::{Path, PathBuf};

use clap::Parser;
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

    #[clap(long, default_value_os_t = default_model_dir())]
    #[serde(default = "default_model_dir")]
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
    #[clap(short, long)]
    pub qdrant_url: Option<String>,

    /// The folder where the qdrant binary is present so we can start the server
    /// and power the qdrant client
    #[clap(short, long)]
    pub qdrant_binary_directory: Option<PathBuf>,
}

impl Configuration {
    /// Directory where logs are written to
    pub fn log_dir(&self) -> PathBuf {
        self.index_dir.join("logs")
    }

    pub fn index_version_mismatch(&self) -> bool {
        // let current: String = read_file_or_default(self.version_file.as_ref().unwrap()).unwrap();

        // !current.is_empty() && current != SCHEMA_VERSION
        false
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

fn default_model_dir() -> PathBuf {
    "model".into()
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
