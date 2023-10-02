/// We want to keep the qdrant binary running here so we can use that along
/// with the client to power our semantic search and everything else which is
/// required.
use std::fs::write;
use std::path::{Path, PathBuf};
use std::{fs::create_dir_all, process::Child, sync::Arc};

use anyhow::Result;

use crate::application::config::configuration::Configuration;

pub struct QdrantServerProcess {
    child: Option<Child>,
    _configuration: Arc<Configuration>,
}

/// This will drop the child process and when it exits, it will kill the process
impl Drop for QdrantServerProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            child.kill().unwrap();
        }
    }
}

impl QdrantServerProcess {
    /// If we already have the qdrant server running, we don't have to start
    /// our own process
    pub async fn initialize(configuration: Arc<Configuration>) -> Result<Self> {
        if configuration.qdrant_storage().exists() {
            return Ok(Self {
                child: None,
                _configuration: configuration,
            });
        }
        let qdrant_location = configuration.qdrant_storage();
        let qd_config_dir = qdrant_location.join("config");
        // Create the directory if it does not exist
        create_dir_all(&qd_config_dir).unwrap();

        // Write out the config file to the proper location
        write(
            qd_config_dir.join("config.yaml"),
            format!(
                include_str!("QDRANT.yaml"),
                storage = &qdrant_location.join("storage").to_string_lossy(),
                snapshots = &qdrant_location.join("snapshots").to_string_lossy()
            ),
        )
        .unwrap();

        // TODO(skcd): What should be the name of the binary here? we need to
        // figure out
        let command = relative_command_path("qdrant_mac").expect("bad bundle");
        let child = Some(run_command(&command, &qdrant_location));

        Ok(Self {
            child,
            _configuration: configuration,
        })
    }
}

fn relative_command_path(command: impl AsRef<str>) -> Option<PathBuf> {
    let cmd = if cfg!(windows) {
        format!("{}.exe", command.as_ref())
    } else {
        command.as_ref().into()
    };

    std::env::current_exe()
        .ok()?
        .parent()
        .map(|dir| dir.join(cmd))
        .filter(|path| path.is_file())
}

#[cfg(unix)]
fn run_command(command: &Path, qdrant_dir: &Path) -> Child {
    use std::process::Command;

    use nix::sys::resource::{getrlimit, setrlimit, Resource};
    use tracing::{error, info};
    match getrlimit(Resource::RLIMIT_NOFILE) {
        Ok((current_soft, current_hard)) if current_hard < 2048 => {
            if let Err(err) = setrlimit(Resource::RLIMIT_NOFILE, 1024, 2048) {
                error!(
                    ?err,
                    new_soft = 1024,
                    new_hard = 2048,
                    current_soft,
                    current_hard,
                    "failed to set rlimit/nofile"
                );
            }
        }
        Ok((current_soft, current_hard)) => {
            info!(current_soft, current_hard, "no change to rlimit needed");
        }
        Err(err) => {
            error!(?err, "failed to get rlimit/nofile");
        }
    }

    // nix::sys::resource::setrlimit().unwrap();
    Command::new(command)
        .current_dir(qdrant_dir)
        .spawn()
        .expect("failed to start qdrant")
}

#[cfg(windows)]
fn run_command(command: &Path, qdrant_dir: &Path) -> Child {
    use std::os::windows::process::CommandExt;

    Command::new(command)
        .current_dir(qdrant_dir)
        // Add a CREATE_NO_WINDOW flag to prevent qdrant console popup
        .creation_flags(0x08000000)
        .spawn()
        .expect("failed to start qdrant")
}

/// We want to wait for qdrant to startup so we can do embedding search etc
pub async fn wait_for_qdrant() {
    use qdrant_client::prelude::*;
    let qdrant =
        // This is hardcoded but its the default which can be found in QDRANT.yaml file
        QdrantClient::new(Some(QdrantClientConfig::from_url("http://127.0.0.1:6334"))).unwrap();

    for _ in 0..60 {
        if qdrant.health_check().await.is_ok() {
            return;
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    panic!("qdrant cannot be started");
}
