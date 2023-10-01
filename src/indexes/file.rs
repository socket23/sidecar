use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{bail, Result};
use async_trait::async_trait;
use tantivy::{schema::Schema, IndexWriter, Term};
use tracing::{info, warn};

use crate::repo::iterator::FileSource;
use crate::{
    application::background::SyncPipes,
    repo::{
        filesystem::{BranchFilter, FileWalker, GitWalker},
        iterator::RepoDirectoryEntry,
        types::{RepoMetadata, RepoRef, Repository},
    },
    state::schema_version::get_schema_version,
};

use super::{
    caching::{CacheKeys, FileCache, FileCacheSnapshot},
    indexer::Indexable,
    schema::File,
};

struct Workload<'a> {
    cache: &'a FileCacheSnapshot<'a>,
    repo_disk_path: &'a Path,
    repo_name: &'a str,
    repo_metadata: &'a RepoMetadata,
    repo_ref: String,
    relative_path: PathBuf,
    normalized_path: PathBuf,
}

impl<'a> Workload<'a> {
    fn cache_keys(&self, dir_entry: &RepoDirectoryEntry) -> CacheKeys {
        let semantic_hash = {
            let mut hash = blake3::Hasher::new();
            hash.update(get_schema_version().as_bytes());
            hash.update(self.relative_path.to_string_lossy().as_ref().as_ref());
            hash.update(self.repo_ref.as_bytes());
            hash.update(dir_entry.buffer().unwrap_or_default().as_bytes());
            hash.finalize().to_hex().to_string()
        };

        let tantivy_hash = {
            let mut hash = blake3::Hasher::new();
            hash.update(semantic_hash.as_ref());
            hash.finalize().to_hex().to_string()
        };

        CacheKeys::new(tantivy_hash)
    }
}

#[async_trait]
impl Indexable for File {
    async fn index_repository(
        &self,
        reporef: &RepoRef,
        repo: &Repository,
        repo_metadata: &RepoMetadata,
        writer: &IndexWriter,
        pipes: &SyncPipes,
    ) -> Result<()> {
        // TODO(skcd): Implement this
        unimplemented!("not implemented this for skcd");
        // let file_cache = Arc::new(FileCache::for_repo(&self.sql, reporef));
        // let cache = file_cache.retrieve().await;
        // let repo_name = reporef.indexed_name();
        // let processed = &AtomicU64::new(0);

        // let file_worker = |count: usize| {
        //     let cache = &cache;
        //     move |dir_entry: RepoDirectoryEntry| {
        //         let completed = processed.fetch_add(1, Ordering::Relaxed);
        //         pipes.index_percent(((completed as f32 / count as f32) * 100f32) as u8);

        //         let entry_disk_path = dir_entry.path().unwrap().to_owned();
        //         let relative_path = {
        //             let entry_srcpath = PathBuf::from(&entry_disk_path);
        //             entry_srcpath
        //                 .strip_prefix(&repo.disk_path)
        //                 .map(ToOwned::to_owned)
        //                 .unwrap_or(entry_srcpath)
        //         };
        //         let normalized_path = repo.disk_path.join(&relative_path);

        //         let workload = Workload {
        //             repo_disk_path: &repo.disk_path,
        //             repo_ref: reporef.to_string(),
        //             repo_name: &repo_name,
        //             relative_path,
        //             normalized_path,
        //             repo_metadata,
        //             cache,
        //         };

        //         if let Err(err) = self.worker(dir_entry, workload, writer) {
        //             warn!(%err, entry_disk_path, "indexing failed; skipping");
        //         }

        //         // TODO(codestory): Enable embedding queue later on
        //         // if let Err(err) = cache.parent().process_embedding_queue() {
        //         //     warn!(?err, "failed to commit embeddings");
        //         // }
        //     }
        // };

        // let start = std::time::Instant::now();

        // // If we could determine the time of the last commit, proceed
        // // with a Git Walker, otherwise use a FS walker
        // if repo_metadata.last_commit_unix_secs.is_some() {
        //     let walker = GitWalker::open_repository(reporef, &repo.disk_path, BranchFilter::Head)?;
        //     let count = walker.len();
        //     walker.for_each(pipes, file_worker(count));
        // } else {
        //     let walker = FileWalker::index_directory(&repo.disk_path);
        //     let count = walker.len();
        //     walker.for_each(pipes, file_worker(count));
        // };

        // if pipes.is_cancelled() {
        //     bail!("cancelled");
        // }

        // // info!(?repo.disk_path, "repo file indexing finished, took {:?}", start.elapsed());

        // file_cache
        //     .synchronize(cache, |key| {
        //         writer.delete_term(Term::from_field_text(self.unique_hash, key));
        //     })
        //     .await?;

        // pipes.index_percent(100);
        // Ok(())
    }

    fn delete_by_repo(&self, writer: &IndexWriter, repo: &Repository) {
        writer.delete_term(Term::from_field_text(
            self.repo_disk_path,
            &repo.disk_path.to_string_lossy(),
        ));
    }

    fn schema(&self) -> Schema {
        self.schema.clone()
    }
}

impl File {
    fn worker(
        &self,
        dir_entry: RepoDirectoryEntry,
        workload: Workload<'_>,
        writer: &IndexWriter,
    ) -> Result<()> {
        let cache_keys = workload.cache_keys(&dir_entry);
        let last_commit = workload.repo_metadata.last_commit_unix_secs.unwrap_or(0);

        // TODO(skcd): Implement this for building the document
        // match dir_entry {
        //     _ if workload.cache.is_fresh(&cache_keys) => {
        //         info!("fresh; skipping");
        //         return Ok(());
        //     }
        //     RepoDirectoryEntry::Dir(dir) => {
        //         let doc = dir.build_document(self, &workload, last_commit, &cache_keys);
        //         writer.add_document(doc)?;
        //     }
        //     RepoDirectoryEntry::File(file) => {
        //         let doc = file
        //             .build_document(
        //                 self,
        //                 &workload,
        //                 &cache_keys,
        //                 last_commit,
        //                 workload.cache.parent(),
        //             )
        //             .ok_or(anyhow::anyhow!("failed to build document"))?;
        //         writer.add_document(doc)?;
        //     }
        //     RepoDirectoryEntry::Other => {
        //         anyhow::bail!("dir entry was neither a file nor a directory")
        //     }
        // }

        Ok(())
    }
}
