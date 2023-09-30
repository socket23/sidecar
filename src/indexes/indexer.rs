use std::ops::Deref;
use std::sync::Arc;
use std::{fs, path::Path};

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use smallvec::SmallVec;
use tantivy::{
    collector::{Collector, MultiFruit},
    schema::Schema,
    tokenizer::NgramTokenizer,
    DocAddress, Document, IndexReader, IndexWriter, Score,
};
use tokio::sync::RwLock;
use tracing::debug;

use crate::application::background::SyncHandle;
use crate::application::config::configuration::Configuration;
use crate::repo::state::{RepoError, RepositoryPool};
use crate::{
    application::background::SyncPipes,
    repo::types::{RepoMetadata, RepoRef, Repository},
};

use super::query::Query;
use super::schema::File;

/// A wrapper around `tantivy::IndexReader`.
///
/// This contains the schema, and also additional fields used to enable re-indexing.
pub struct Indexer<T> {
    pub source: T,
    pub index: tantivy::Index,
    pub reader: RwLock<IndexReader>,
    pub reindex_buffer_size: usize,
    pub reindex_threads: usize,
}

#[async_trait]
pub trait Indexable: Send + Sync {
    /// This is where files are scanned and indexed.
    async fn index_repository(
        &self,
        reporef: &RepoRef,
        repo: &Repository,
        metadata: &RepoMetadata,
        writer: &IndexWriter,
        pipes: &SyncPipes,
    ) -> Result<()>;

    fn delete_by_repo(&self, writer: &IndexWriter, repo: &Repository);

    /// Return the tantivy `Schema` of the current index
    fn schema(&self) -> Schema;
}

// This is the inner most index write handle, so we can use this to index the
// inner caller and make that work
// the writer here is the tantivy writer which we need to use
pub struct IndexWriteHandle<'a> {
    source: &'a dyn Indexable,
    index: &'a tantivy::Index,
    reader: &'a RwLock<IndexReader>,
    writer: IndexWriter,
}

impl<'a> IndexWriteHandle<'a> {
    pub async fn refresh_reader(&self) -> Result<()> {
        *self.reader.write().await = self.index.reader()?;
        Ok(())
    }

    pub fn delete(&self, repo: &Repository) {
        self.source.delete_by_repo(&self.writer, repo)
    }

    pub async fn index(
        &self,
        reporef: &RepoRef,
        repo: &Repository,
        metadata: &RepoMetadata,
        progress: &SyncPipes,
    ) -> Result<()> {
        self.source
            .index_repository(reporef, repo, metadata, &self.writer, progress)
            .await
    }

    pub async fn commit(&mut self) -> Result<()> {
        self.writer.commit()?;
        self.refresh_reader().await?;

        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        self.writer.rollback()?;
        Ok(())
    }
}

#[async_trait]
pub trait DocumentRead: Send + Sync {
    type Schema;
    type Document;

    /// Return whether this reader can process this query.
    fn query_matches(&self, query: &Query<'_>) -> bool;

    /// Compile a set of parsed queries into a single `tantivy` query.
    fn compile<'a, I>(
        &self,
        schema: &Self::Schema,
        queries: I,
        index: &tantivy::Index,
    ) -> Result<Box<dyn tantivy::query::Query>>
    where
        I: Iterator<Item = &'a Query<'a>>;

    /// Read a tantivy document into the specified output type.
    fn read_document(&self, schema: &Self::Schema, doc: Document) -> Self::Document;
}

impl<T: Indexable> Indexer<T> {
    fn write_handle(&self) -> Result<IndexWriteHandle<'_>> {
        Ok(IndexWriteHandle {
            source: &self.source,
            index: &self.index,
            reader: &self.reader,
            writer: self
                .index
                .writer_with_num_threads(self.reindex_threads, self.reindex_buffer_size)?,
        })
    }

    fn init_index(schema: Schema, path: &Path, threads: usize) -> Result<tantivy::Index> {
        fs::create_dir_all(path).context("failed to create index dir")?;

        let mut index =
            tantivy::Index::open_or_create(tantivy::directory::MmapDirectory::open(path)?, schema)?;

        index.set_default_multithread_executor()?;
        index.set_multithread_executor(threads)?;
        index
            .tokenizers()
            .register("default", NgramTokenizer::new(1, 3, false)?);

        Ok(index)
    }

    /// Create an index using `source` at the specified path.
    pub fn create(source: T, path: &Path, buffer_size: usize, threads: usize) -> Result<Self> {
        let index = Self::init_index(source.schema(), path, threads)?;
        let reader = index.reader()?.into();
        let instance = Self {
            reader,
            index,
            source,
            reindex_threads: threads,
            reindex_buffer_size: buffer_size,
        };

        Ok(instance)
    }

    pub async fn query<'a, R, I, C>(
        &'a self,
        queries: I,
        doc_reader: &'a R,
        collector: C,
    ) -> Result<SearchResults<'_, R::Document>>
    where
        I: Iterator<Item = &'a Query<'a>> + Send,
        C: Collector<Fruit = (Vec<(Score, DocAddress)>, MultiFruit)>,
        R: DocumentRead<Schema = T>,
    {
        let searcher = self.reader.read().await.searcher();
        let queries = queries
            .filter(|q| doc_reader.query_matches(q))
            .collect::<SmallVec<[_; 2]>>();
        let compiled_query =
            doc_reader.compile(&self.source, queries.iter().copied(), &self.index)?;

        let (top_k, metadata) = searcher
            .search(&compiled_query, &collector)
            .context("failed to execute search query")?;

        let iter = top_k.into_iter().map(move |(_score, addr)| {
            let doc = searcher.doc(addr).unwrap();
            doc_reader.read_document(&self.source, doc)
        });

        Ok(SearchResults {
            docs: Box::new(iter),
            metadata,
        })
    }
}

pub type WriteHandleForIndexersRef<'a> = [IndexWriteHandle<'a>];

pub struct WriteHandleForIndexers<'a> {
    handles: Vec<IndexWriteHandle<'a>>,
    _write_lock: tokio::sync::MutexGuard<'a, ()>,
}

impl<'a> Deref for WriteHandleForIndexers<'a> {
    type Target = WriteHandleForIndexersRef<'a>;

    fn deref(&self) -> &Self::Target {
        &self.handles
    }
}

impl<'a> WriteHandleForIndexers<'a> {
    pub async fn commit(self) -> Result<()> {
        for mut handle in self.handles {
            handle.commit().await?
        }

        Ok(())
    }

    pub async fn index(
        &self,
        sync_handle: &SyncHandle,
        repo: &Repository,
    ) -> Result<Arc<RepoMetadata>, RepoError> {
        let metadata = repo.get_repo_metadata().await;

        futures::future::join_all(self.handles.iter().map(|handle| {
            handle.index(&sync_handle.reporef, repo, &metadata, sync_handle.pipes())
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

        Ok(metadata)
    }

    pub fn rollback(self) -> Result<()> {
        for mut handle in self.handles {
            handle.rollback()?;
        }
        Ok(())
    }
}

pub struct SearchResults<'a, T> {
    pub docs: Box<dyn Iterator<Item = T> + Sync + Send + 'a>,
    pub metadata: MultiFruit,
}

pub struct Indexes {
    pub file: Indexer<File>,
    write_mutex: tokio::sync::Mutex<()>,
}

impl Indexes {
    pub async fn new(repo_pool: RepositoryPool, config: Arc<Configuration>) -> Result<Self> {
        // Figure out how to do version mismatch
        // if config.state_source.index_version_mismatch() {
        //     // we don't support old schemas, and tantivy will hard
        //     // error if we try to open a db with a different schema.
        //     std::fs::remove_dir_all(config.index_path("repo"))?;
        //     std::fs::remove_dir_all(config.index_path("content"))?;

        //     let mut refs = vec![];
        //     // knocking out our current file caches will force re-indexing qdrant
        //     repo_pool.for_each(|reporef, repo| {
        //         refs.push(reporef.to_owned());
        //         repo.last_index_unix_secs = 0;
        //     });

        //     for reporef in refs {
        //         FileCache::for_repo(&sql, semantic.as_ref(), &reporef)
        //             .delete()
        //             .await?;
        //     }

        //     if let Some(ref semantic) = semantic {
        //         semantic.reset_collection_blocking().await?;
        //     }
        // }
        // config.source.save_index_version()?;

        Ok(Self {
            file: Indexer::create(
                File::new(),
                config.index_path("content").as_ref(),
                config.buffer_size,
                config.max_threads,
            )?,
            write_mutex: Default::default(),
        })
    }

    pub async fn writers(&self) -> Result<WriteHandleForIndexers<'_>> {
        let id: u64 = rand::random();
        debug!(id, "waiting for other writers to finish");
        let _write_lock = self.write_mutex.lock().await;
        debug!(id, "lock acquired");

        Ok(WriteHandleForIndexers {
            handles: vec![self.file.write_handle()?],
            _write_lock,
        })
    }
}
