use std::path::PathBuf;

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
}

impl Configuration {
    /// Directory where logs are written to
    pub fn log_dir(&self) -> PathBuf {
        self.index_dir.join("logs")
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
