// We are going to store caches here for what we have indexed already
// and what we still have to index
//
// Now the way we want to go about doing this:
// we use a fs based system and wrap it in a lock so we are okay with things

use std::path::PathBuf;

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// This is the storage for the underlying struct which we will use to store
/// anything and everything
pub struct FSStorage<T: Serialize + DeserializeOwned> {
    source: T,
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl<T: Serialize + DeserializeOwned> FSStorage<T> {
    pub fn new(source: T, path: PathBuf) -> Self {
        Self {
            source,
            path,
            write_lock: Mutex::new(()),
        }
    }

    // This will store the underlying data to the path we are interested in
    pub async fn store_to_path(&self) -> Result<()> {
        // We take the lock here, since we want to be the only ones writing
        // to it for correctness sake
        let _lock = self.write_lock.lock().await;
        tokio::fs::create_dir_all(self.path.parent().unwrap()).await?;
        let mut file = tokio::fs::File::create(&self.path).await?;
        let data = serde_json::to_vec(&self.source)?;
        file.write_all(&data).await?;
        Ok(())
    }
}
