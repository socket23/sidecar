// We are going to try and use tanvity here to figure out if we can do some
// kind of parsing here and keep the goodness of search going
// our goal is to power lexical search and other kind of searches here

// use std::fs;
// use std::path::Path;

// use anyhow::{Context, Result};
// use async_trait::async_trait;
// use tantivy::collector::Collector;
// use tantivy::collector::MultiFruit;
// use tantivy::query::Query;
// use tantivy::schema::Field;
// use tantivy::schema::Schema;
// use tantivy::DocAddress;
// use tantivy::Document;
// use tantivy::IndexReader;
// use tantivy::IndexWriter;
// use tantivy::Score;

// use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
// use tantivy::tokenizer::NgramTokenizer;
// use tokio::sync::RwLock;

// #[async_trait]
// pub trait DocumentRead: Send + Sync {
//     type Schema;
//     type Document;

//     /// Return whether this reader can process this query.
//     fn query_matches(&self, query: &Query<'_>) -> bool;

//     /// Compile a set of parsed queries into a single `tantivy` query.
//     fn compile<'a, I>(
//         &self,
//         schema: &Self::Schema,
//         queries: I,
//         index: &tantivy::Index,
//     ) -> Result<Box<dyn tantivy::query::Query>>
//     where
//         I: Iterator<Item = &'a Query<'a>>;

//     /// Read a tantivy document into the specified output type.
//     fn read_document(&self, schema: &Self::Schema, doc: Document) -> Self::Document;
// }

// #[derive(Clone)]
// pub struct File {
//     pub(super) schema: Schema,
//     // pub(super) semantic: Option<Semantic>,
//     #[cfg(feature = "debug")]
//     pub histogram: Arc<RwLock<Histogram>>,

//     /// Unique ID for the file in a repo
//     pub unique_hash: Field,

//     /// Path to the root of the repo on disk
//     pub repo_disk_path: Field,
//     /// Path to the file, relative to the repo root
//     pub relative_path: Field,

//     /// Unique repo identifier, of the form:
//     ///  local: local//path/to/repo
//     /// github: github.com/org/repo
//     pub repo_ref: Field,

//     /// Indexed repo name, of the form:
//     ///  local: repo
//     /// github: github.com/org/repo
//     pub repo_name: Field,

//     pub content: Field,
//     pub line_end_indices: Field,

//     /// a flat list of every symbol's text, for searching, e.g.:
//     /// ["File", "Repo", "worker"]
//     pub symbols: Field,
//     pub symbol_locations: Field,

//     /// fast fields for scoring
//     pub lang: Field,
//     pub avg_line_length: Field,
//     pub last_commit_unix_seconds: Field,

//     /// fast byte versions of certain fields for collector-level filtering
//     pub raw_content: Field,
//     pub raw_repo_name: Field,
//     pub raw_relative_path: Field,

//     /// list of branches in which this file can be found
//     pub branches: Field,

//     /// Whether this entry is a file or a directory
//     pub is_directory: Field,
// }

// /// An index representing a repository to allow free-text search on
// /// repository names
// pub struct Repo {
//     pub(super) schema: Schema,

//     /// Path to the root of the repo on disk
//     pub disk_path: Field,

//     /// Name of the org
//     pub org: Field,

//     /// Indexed repo name, of the form:
//     ///  local: repo
//     /// github: github.com/org/repo
//     pub name: Field,
//     pub raw_name: Field,

//     /// Unique repo identifier, of the form:
//     ///  local: local//path/to/repo
//     /// github: github.com/org/repo
//     pub repo_ref: Field,
// }

// pub struct Indexes {
//     pub repo: Indexer<Repo>,
//     pub file: Indexer<File>,
//     write_mutex: tokio::sync::Mutex<()>,
// }

// /// A wrapper around `tantivy::IndexReader`.
// ///
// /// This contains the schema, and also additional fields used to enable re-indexing.
// pub struct Indexer<T> {
//     pub source: T,
//     pub index: tantivy::Index,
//     pub reader: RwLock<IndexReader>,
//     pub reindex_buffer_size: usize,
//     pub reindex_threads: usize,
// }

// #[async_trait]
// pub trait Indexable: Send + Sync {
//     /// This is where files are scanned and indexed.
//     async fn index_repository(
//         &self,
//         reporef: &RepoRef,
//         repo: &Repository,
//         metadata: &RepoMetadata,
//         writer: &IndexWriter,
//         pipes: &SyncPipes,
//     ) -> Result<()>;

//     fn delete_by_repo(&self, writer: &IndexWriter, repo: &Repository);

//     /// Return the tantivy `Schema` of the current index
//     fn schema(&self) -> Schema;
// }

// pub struct IndexWriteHandle<'a> {
//     source: &'a dyn Indexable,
//     index: &'a tantivy::Index,
//     reader: &'a RwLock<IndexReader>,
//     writer: IndexWriter,
// }

// impl<'a> IndexWriteHandle<'a> {
//     pub async fn refresh_reader(&self) -> Result<()> {
//         *self.reader.write().await = self.index.reader()?;
//         Ok(())
//     }

//     pub fn delete(&self, repo: &Repository) {
//         self.source.delete_by_repo(&self.writer, repo)
//     }

//     pub async fn index(
//         &self,
//         reporef: &RepoRef,
//         repo: &Repository,
//         metadata: &RepoMetadata,
//         progress: &SyncPipes,
//     ) -> Result<()> {
//         self.source
//             .index_repository(reporef, repo, metadata, &self.writer, progress)
//             .await
//     }

//     pub async fn commit(&mut self) -> Result<()> {
//         self.writer.commit()?;
//         self.refresh_reader().await?;

//         Ok(())
//     }

//     pub fn rollback(&mut self) -> Result<()> {
//         self.writer.rollback()?;
//         Ok(())
//     }
// }

// impl<T: Indexable> Indexer<T> {
//     fn write_handle(&self) -> Result<IndexWriteHandle<'_>> {
//         Ok(IndexWriteHandle {
//             source: &self.source,
//             index: &self.index,
//             reader: &self.reader,
//             writer: self
//                 .index
//                 .writer_with_num_threads(self.reindex_threads, self.reindex_buffer_size)?,
//         })
//     }

//     fn init_index(schema: Schema, path: &Path, threads: usize) -> Result<tantivy::Index> {
//         fs::create_dir_all(path).context("failed to create index dir")?;

//         let mut index =
//             tantivy::Index::open_or_create(tantivy::directory::MmapDirectory::open(path)?, schema)?;

//         index.set_default_multithread_executor()?;
//         index.set_multithread_executor(threads)?;
//         index
//             .tokenizers()
//             .register("default", NgramTokenizer::new(1, 3, false));

//         Ok(index)
//     }

//     /// Create an index using `source` at the specified path.
//     pub fn create(source: T, path: &Path, buffer_size: usize, threads: usize) -> Result<Self> {
//         let index = Self::init_index(source.schema(), path, threads)?;
//         let reader = index.reader()?.into();
//         let instance = Self {
//             reader,
//             index,
//             source,
//             reindex_threads: threads,
//             reindex_buffer_size: buffer_size,
//         };

//         Ok(instance)
//     }

//     pub async fn query<'a, R, I, C>(
//         &'a self,
//         queries: I,
//         doc_reader: &'a R,
//         collector: C,
//     ) -> Result<SearchResults<'_, R::Document>>
//     where
//         I: Iterator<Item = &'a Query<'a>> + Send,
//         C: Collector<Fruit = (Vec<(Score, DocAddress)>, MultiFruit)>,
//         R: DocumentRead<Schema = T>,
//     {
//         let searcher = self.reader.read().await.searcher();
//         let queries = queries
//             .filter(|q| doc_reader.query_matches(q))
//             .collect::<Vec<[_; 2]>>();
//         let compiled_query =
//             doc_reader.compile(&self.source, queries.iter().copied(), &self.index)?;

//         let (top_k, metadata) = searcher
//             .search(&compiled_query, &collector)
//             .context("failed to execute search query")?;

//         let iter = top_k.into_iter().map(move |(_score, addr)| {
//             let doc = searcher.doc(addr).unwrap();
//             doc_reader.read_document(&self.source, doc)
//         });

//         Ok(SearchResults {
//             docs: Box::new(iter),
//             metadata,
//         })
//     }
// }

// pub struct SearchResults<'a, T> {
//     pub docs: Box<dyn Iterator<Item = T> + Sync + Send + 'a>,
//     pub metadata: MultiFruit,
// }
