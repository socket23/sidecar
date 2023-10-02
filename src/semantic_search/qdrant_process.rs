/// We want to keep the qdrant binary running here so we can use that along
/// with the client to power our semantic search and everything else which is
/// required.
use std::{process::Child, sync::Arc};

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
