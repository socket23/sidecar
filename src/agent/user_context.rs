//! We are going to implement how tht agent is going to use user context

use std::sync::Arc;

use anyhow::Result;
use futures::stream;
use futures::StreamExt;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::QueryParser;
use tantivy::tokenizer::NgramTokenizer;

use crate::agent::llm_funcs;
use crate::agent::prompts;
use crate::agent::types::AgentAction;
use crate::application::application::Application;
use crate::chunking::languages::TSLanguageParsing;
use crate::indexes::indexer::Indexer;
use crate::indexes::schema::CodeSnippet;
use crate::indexes::schema::CodeSnippetTokenizer;
use crate::indexes::schema::QuickCodeSnippet;
use crate::indexes::schema::QuickCodeSnippetDocument;
use crate::webserver::agent::FileContentValue;

use super::{llm_funcs::LlmClient, types::Agent};

impl Agent {
    pub async fn truncate_user_context(&mut self, query: &str) -> Result<String> {
        // We get different levels of context here from the user:
        // - @file full files (which we have to truncate and get relevant values)
        // - @selection selection ranges from the user (these we have to include including expanding them a bit on each side)
        // - @code code symbols which the user is interested in, which we also keep as it is cause they might be useful
        // so our rough maths is as follows:
        // - @selection (we always keep)
        // - @code (we keep unless its a class in which case we can truncate it a bit)
        // - @file (we have to truncate if it does not fit in the context window)
        let user_context = self
            .user_context
            .as_ref()
            .expect("user_context to be there")
            .clone();
        let user_variables = user_context.variables;
        let user_files = user_context.file_content_map;

        let self_ = &*self;
        let lexical_search_and_file = stream::iter(user_files)
            .map(|file_value| async move {
                let language = &file_value.language;
                // First generate the outline for the file here
                let language_config = self_.application.language_parsing.for_lang(language);
                let file_content = &file_value.file_content;
                // Now we can query gpt3.5 with the output here and aks it to
                // generate the code search keywords which we need
                let system_prompt = prompts::lexical_search_system_prompt(
                    language_config.map(|lang_config| {
                        lang_config.generate_file_outline(file_content.as_bytes())
                    }),
                    &file_value.file_path,
                );
                let functions = serde_json::from_value::<Vec<llm_funcs::llm::Function>>(
                    prompts::lexical_search_functions(),
                )
                .unwrap();
                let response = self_
                    .get_llm_client()
                    .stream_function_call(
                        llm_funcs::llm::OpenAIModel::GPT3_5_16k,
                        vec![
                            llm_funcs::llm::Message::system(&system_prompt),
                            llm_funcs::llm::Message::user(query),
                            llm_funcs::llm::Message::user("CALL A FUNCTION!. Do not answer"),
                        ],
                        functions,
                        0.0,
                        Some(0.2),
                    )
                    .await;
                if let Ok(Some(response)) = response {
                    let agent_action =
                        AgentAction::from_gpt_response(&response).map(|response| Some(response));
                    match agent_action {
                        Ok(Some(AgentAction::Code { query })) => {
                            // If we match the code output we are good, otherwise
                            // we messed up in the pipeline somewhere
                            return Some((query, file_value));
                        }
                        _ => return None,
                    }
                } else {
                    None
                }
            })
            .buffer_unordered(10)
            .filter_map(|value| async { value })
            .collect::<Vec<_>>()
            .await;

        // // First we create an in-memory tantivy index to perform search
        // let code_snippet_indexer = Indexer::create_for_in_memory_code_search(
        //     CodeSnippet::new(
        //         self.application.sql.clone(),
        //         self.application.language_parsing.clone(),
        //     ),
        //     100_000_000,
        //     std::thread::available_parallelism().unwrap().get(),
        // )
        // .expect("index creation to not fail");

        // we want to be fast af, so let's parallelize the lexical search on each
        // of these files and get the queries
        // stream::iter(user_files.into_iter())
        //     .map(|value| {
        //         let fs_file_path = value.file_path;
        //         let file_content = value.file_content;
        //     })
        //     .collect()
        //     .await;
        unimplemented!();
    }
}

struct QuickCodeSnippetIndex {
    schema: QuickCodeSnippet,
    index: tantivy::Index,
    reader: tantivy::IndexReader,
    writer: tantivy::IndexWriter,
}

impl QuickCodeSnippetIndex {
    fn create_in_memory_index(
        schema: &tantivy::schema::Schema,
        language_parsing: Arc<TSLanguageParsing>,
    ) -> Self {
        let mut index = tantivy::Index::create_in_ram(QuickCodeSnippet::new().schema);
        index
            .set_default_multithread_executor()
            .expect("setting multi-thread executor to not fail");
        index
            .set_multithread_executor(std::thread::available_parallelism().unwrap().get())
            .expect("setting threads to not fail");
        index.tokenizers().register(
            "default",
            NgramTokenizer::new(1, 3, false).expect("ngram tokenizer to work"),
        );
        index
            .tokenizers()
            .register("code_snippet", CodeSnippetTokenizer {});
        let reader = index.reader().expect("reader to not fail");
        let writer = index
            .writer_with_num_threads(
                std::thread::available_parallelism().unwrap().get(),
                100_000_000,
            )
            .expect("index writer to not fail");
        Self {
            schema: QuickCodeSnippet::new(),
            index,
            reader,
            writer,
        }
    }

    // TODO(skcd): Implement this
    fn lexical_query(&self, file_path: &str, query: &str) -> Vec<QuickCodeSnippetDocument> {
        let searcher = self.reader.searcher();
        let collector = TopDocs::with_limit(20);
        let code_snippet_source = &self.schema;
        let query_parser = QueryParser::for_index(
            searcher.index(),
            vec![code_snippet_source.path, code_snippet_source.content],
        );
        let tokens = CodeSnippetTokenizer::tokenize_call(query);
        let mut query_string = tokens
            .iter()
            .map(|token| format!(r#"content:{}""#, token.text))
            .collect::<Vec<_>>()
            .join(" OR ");
        query_string = format!(r#"path:"{}" AND ({})"#, file_path, query_string);
        let query = query_parser
            .parse_query(query_string.as_str())
            .expect("parsing query should not fail");
        let top_docs = searcher
            .search(&query, &collector)
            .expect("top docs collection should not fail");
        top_docs
            .into_iter()
            .map(|doc| {
                let retrieved_doc = searcher
                    .doc(doc.1)
                    .expect("failed to get document by the address");
                QuickCodeSnippetDocument::read_document_with_score(
                    &self.schema,
                    retrieved_doc,
                    doc.0,
                )
            })
            .collect::<Vec<_>>()
    }
}

/// We can build the tantivy documents this way
impl FileContentValue {
    fn build_documents(
        mut self,
        schema: &QuickCodeSnippet,
        language_parsing: Arc<TSLanguageParsing>,
    ) -> Vec<tantivy::schema::Document> {
        let chunks = language_parsing
            .chunk_file(
                &self.file_path,
                &self.file_content,
                None,
                Some(&self.language),
            )
            .into_iter()
            .filter(|span| span.data.is_some())
            .collect::<Vec<_>>();
        chunks
            .into_iter()
            .map(|chunk| {
                let data = chunk.data.unwrap_or_default();
                doc!(
                    schema.start_line => chunk.start as u64,
                    schema.end_line => chunk.end as u64,
                    schema.path => self.file_path.to_owned(),
                    schema.content =>  data.to_owned(),
                )
            })
            .collect::<Vec<_>>()
    }
}

// Now we want to build the index over here
fn build_tantivy_index(
    mut quick_code_snippet_index: QuickCodeSnippetIndex,
    file_content_value: Vec<FileContentValue>,
    application: Arc<Application>,
) -> QuickCodeSnippetIndex {
    let _ = file_content_value
        .into_par_iter()
        .for_each(|file_content_value| {
            let documents = file_content_value.build_documents(
                &QuickCodeSnippet::new(),
                application.language_parsing.clone(),
            );
            documents.into_iter().for_each(|document| {
                let _ = quick_code_snippet_index.writer.add_document(document);
            });
        });
    quick_code_snippet_index
        .writer
        .commit()
        .expect("commit to not fail");
    quick_code_snippet_index
        .reader
        .reload()
        .expect("reload to not fail");
    // Now our quick code snippet index is ready for search
    quick_code_snippet_index
}

async fn rank_spans_on_embeddings(
    fs_file_path: String,
    file_content: String,
    language: String,
    query: String,
    application: Arc<Application>,
) -> Vec<QuickCodeSnippetDocument> {
    if application.semantic_client.is_none() {
        return vec![];
    }
    let embedder = application
        .semantic_client
        .clone()
        .expect("is_none check above to hold")
        .get_embedder();
    let query_embeddings = embedder.embed(&query).expect("embedding to not fail");
    let chunks = application
        .language_parsing
        .chunk_file(&fs_file_path, &file_content, None, None)
        .into_iter()
        .filter(|chunk| chunk.data.is_some())
        .collect::<Vec<_>>();
    let embedded_values = embedder
        .batch_embed(
            chunks
                .iter()
                .map(|chunk| chunk.data.as_ref().expect("data to be present").as_str())
                .collect::<Vec<_>>(),
        )
        .await
        .expect("embedding generation to not fail");
    let filtered_chunks_len = chunks.len();
    let mut final_generation = vec![];
    for index in 0..filtered_chunks_len {
        let metric = floating_distance::Metric::Cosine;
        let score = metric.measure::<f32>(&embedded_values[index], &query_embeddings);
        // we want to compute cosine similarity here between the vectors we are getting
        final_generation.push(QuickCodeSnippetDocument::new(
            fs_file_path.clone(),
            chunks[index]
                .data
                .as_ref()
                .expect("data to be present")
                .to_owned(),
            chunks[index].start as u64,
            chunks[index].end as u64,
            score,
        ));
    }
    final_generation
}
