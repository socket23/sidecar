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

use crate::{
    application::background::SyncPipes,
    repo::types::{RepoMetadata, RepoRef, Repository},
};

use super::query::Query;

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

pub struct IndexWriteHandle<'a> {
    source: &'a dyn Indexable,
    index: &'a tantivy::Index,
    reader: &'a RwLock<IndexReader>,
    writer: IndexWriter,
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

pub struct SearchResults<'a, T> {
    pub docs: Box<dyn Iterator<Item = T> + Sync + Send + 'a>,
    pub metadata: MultiFruit,
}
