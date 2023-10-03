// We are going to store caches here for what we have indexed already
// and what we still have to index
//
// Now the way we want to go about doing this:
// we use a fs based system and wrap it in a lock so we are okay with things

use scc::hash_map::Entry;
use sqlx::Sqlite;
use std::collections::HashSet;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::db::sqlite::SqlDb;
use crate::repo::types::RepoRef;

// Indexes might require their own keys, we know tantivy is fucked because
// it expects a key which is unique to the doc schema which you put in...
// so if we query it with the wrong schema it blows in your face :|
// for now we care about tantivy so lets get that working
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct CacheKeys {
    tantivy: String,
    // // We also want to store the file content hash as a cache key, so we can
    // // evict the ones which we are no longer interested in
    // file_content_hash: String,
}

impl CacheKeys {
    pub fn new(tantivy: String) -> Self {
        Self { tantivy }
    }

    pub fn tantivy(&self) -> &str {
        &self.tantivy
    }

    // pub fn file_content_hash(&self) -> &str {
    //     &self.file_content_hash
    // }
}

#[derive(serde::Serialize, serde::Deserialize, Eq)]
pub struct FreshValue<T> {
    // default value is `false` on deserialize
    pub(crate) fresh: bool,
    pub(crate) value: T,
}

impl<T: Default> FreshValue<T> {
    fn fresh_default() -> Self {
        Self {
            fresh: true,
            value: Default::default(),
        }
    }
}

impl<T> PartialEq for FreshValue<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<T> FreshValue<T> {
    fn stale(value: T) -> Self {
        Self {
            fresh: false,
            value,
        }
    }
}

impl<T> From<T> for FreshValue<T> {
    fn from(value: T) -> Self {
        Self { fresh: true, value }
    }
}

/// This is the storage for the underlying struct which we will use to store
/// anything and everything
pub struct FSStorage<T: Serialize + DeserializeOwned + PartialEq> {
    source: T,
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl<T: Serialize + DeserializeOwned + PartialEq> FSStorage<T> {
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

pub struct FileCacheSnapshot<'a> {
    snapshot: Arc<scc::HashMap<CacheKeys, FreshValue<()>>>,
    parent: &'a FileCache<'a>,
}

impl<'a> Deref for FileCacheSnapshot<'a> {
    type Target = scc::HashMap<CacheKeys, FreshValue<()>>;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}

impl<'a> FileCacheSnapshot<'a> {
    pub fn parent(&self) -> &'a FileCache<'a> {
        self.parent
    }

    pub fn is_fresh(&self, keys: &CacheKeys) -> bool {
        match self.snapshot.entry(keys.clone()) {
            Entry::Occupied(mut val) => {
                val.get_mut().fresh = true;

                true
            }
            Entry::Vacant(val) => {
                _ = val.insert_entry(FreshValue::fresh_default());

                false
            }
        }
    }
}

// This is where we maintain a cache of the file and have a storage layer
// backing up the cache and everything happening here
pub struct FileCache<'a> {
    sqlite: &'a SqlDb,
    reporef: &'a RepoRef,
    // semantic: Option<&'a Semantic>,
    // embed_queue: EmbedQueue,
}

impl<'a> FileCache<'a> {
    pub fn for_repo(sqlite: &'a SqlDb, reporef: &'a RepoRef) -> Self {
        Self { sqlite, reporef }
    }

    // Retrieve a file-level snapshot of the cache for the repository in scope.
    pub(crate) async fn retrieve(&'a self) -> FileCacheSnapshot<'a> {
        let repo_str = self.reporef.to_string();
        let rows = sqlx::query! {
            "SELECT tantivy_cache_key FROM file_cache \
             WHERE repo_ref = ?",
            repo_str,
        }
        .fetch_all(self.sqlite.as_ref())
        .await;

        let output = scc::HashMap::default();
        for row in rows.into_iter().flatten() {
            let tantivy_cache_key = row.tantivy_cache_key;
            _ = output.insert(
                CacheKeys {
                    tantivy: tantivy_cache_key,
                },
                FreshValue::stale(()),
            );
        }

        FileCacheSnapshot {
            snapshot: output.into(),
            parent: self,
        }
    }

    async fn delete_files(&self, tx: &mut sqlx::Transaction<'_, Sqlite>) -> anyhow::Result<()> {
        let repo_str = self.reporef.to_string();
        sqlx::query! {
            "DELETE FROM file_cache \
                 WHERE repo_ref = ?",
            repo_str
        }
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn synchronize(
        &'a self,
        cache: FileCacheSnapshot<'a>,
        _delete_tantivy: impl Fn(&str),
    ) -> anyhow::Result<()> {
        let mut tx = self.sqlite.begin().await?;
        self.delete_files(&mut tx).await?;

        // files that are no longer tracked by the git index are to be removed
        // from the tantivy & qdrant indices
        // let qdrant_stale = {
        //     let mut semantic_fresh = HashSet::new();
        //     let mut semantic_all = HashSet::new();

        //     cache.retain(|k, v| {
        //         // check if it's already in to avoid unnecessary copies
        //         if v.fresh && !semantic_fresh.contains(k.semantic()) {
        //             semantic_fresh.insert(k.semantic().to_string());
        //         }

        //         if !semantic_all.contains(k.semantic()) {
        //             semantic_all.insert(k.semantic().to_string());
        //         }

        //         // just call the passed closure for tantivy
        //         if !v.fresh {
        //             delete_tantivy(k.tantivy())
        //         }

        //         v.fresh
        //     });

        //     semantic_all
        //         .difference(&semantic_fresh)
        //         .cloned()
        //         .collect::<Vec<_>>()
        // };

        // generate a transaction to push the remaining entries
        // into the sql cache
        {
            let mut next = cache.first_occupied_entry_async().await;
            while let Some(entry) = next {
                let key = entry.key();
                let tantivy_key = key.tantivy().to_owned();
                let repo_str = self.reporef.to_string();
                sqlx::query!(
                    "INSERT INTO file_cache \
		 (repo_ref, tantivy_cache_key) \
                 VALUES (?, ?)",
                    repo_str,
                    tantivy_key,
                )
                .execute(&mut *tx)
                .await?;

                next = entry.next();
            }

            tx.commit().await?;
        }

        // batch-delete points from qdrant index
        // if !qdrant_stale.is_empty() {
        //     if let Some(semantic) = self.semantic {
        //         let semantic = semantic.clone();
        //         let reporef = self.reporef.to_string();
        //         tokio::spawn(async move {
        //             semantic
        //                 .delete_points_for_hash(reporef.as_str(), qdrant_stale.into_iter())
        //                 .await;
        //         });
        //     }
        // }

        // make sure we generate & commit all remaining embeddings
        // self.batched_embed_or_flush_queue(true).await?;

        Ok(())
    }

    // TODO(skcd): Start processing the chunks here
    // We have to process the chunks here and keep cache properly
    pub async fn process_chunks(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
