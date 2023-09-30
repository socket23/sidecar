use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{bail, Result};
use async_trait::async_trait;
use tantivy::{schema::Schema, IndexWriter, Term};
use tracing::info;

use crate::{
    application::background::SyncPipes,
    repo::{
        filesystem::FileWalker,
        types::{RepoMetadata, RepoRef, Repository},
    },
};

use super::{indexer::Indexable, schema::File};

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
        // TODO(skcd): Pick up from here and get the indexing with tantivy done
        unimplemented!("File::index_repository");
        // let file_cache = Arc::new(FileCache::for_repo(
        //     &self.sql,
        //     self.semantic.as_ref(),
        //     reporef,
        // ));
        // let cache = file_cache.retrieve().await;
        // let repo_name = reporef.indexed_name();
        // let processed = &AtomicU64::new(0);

        // let file_worker = |count: usize| {
        //     let cache = &cache;
        //     move |dir_entry: RepoDirEntry| {
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

        //         trace!(entry_disk_path, "queueing entry");
        //         if let Err(err) = self.worker(dir_entry, workload, writer) {
        //             warn!(%err, entry_disk_path, "indexing failed; skipping");
        //         }

        //         if let Err(err) = cache.parent().process_embedding_queue() {
        //             warn!(?err, "failed to commit embeddings");
        //         }
        //     }
        // };

        // let start = std::time::Instant::now();

        // // If we could determine the time of the last commit, proceed
        // // with a Git Walker, otherwise use a FS walker
        // if repo_metadata.last_commit_unix_secs.is_some() {
        //     let walker = GitWalker::open_repository(
        //         reporef,
        //         &repo.disk_path,
        //         repo.branch_filter.as_ref().map(Into::into),
        //     )?;
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

        // info!(?repo.disk_path, "repo file indexing finished, took {:?}", start.elapsed());

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
