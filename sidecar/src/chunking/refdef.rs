use std::ops::Not;
use std::sync::Arc;

use tantivy::{
    collector::TopDocs,
    query::{BooleanQuery, TermQuery},
    schema::IndexRecordOption,
    Term,
};

use crate::{
    indexes::{file::ContentDocument, indexer::Indexes, query::trigrams},
    repo::types::RepoRef,
};

use super::{
    languages::TSLanguageParsing,
    navigation::{FileSymbols, Occurrence, OccurrenceKind},
    scope_graph::{NodeKind, Snipper},
    text_document::Range,
};

/// Ref/def allows us to get reference and go to definition
/// without using the LSP
/// This uses tantivy along with tree-sitter to figure out where the refernces
/// and the defintiions are
pub async fn refdef(
    indexes: Arc<Indexes>,
    repo_ref: &RepoRef,
    hovered_text: &str,
    text_range: &Range,
    source_document: &ContentDocument,
    language: &str,
    language_parsing: Arc<TSLanguageParsing>,
) -> anyhow::Result<Vec<FileSymbols>> {
    // produce search based results here
    let regex_str = regex::escape(hovered_text);
    let target = regex::Regex::new(&format!(r"\b{regex_str}\b")).expect("failed to build regex");
    // perform a text search for hovered_text
    let file_source = &indexes.file.source;
    let indexer = &indexes.file;
    let query = {
        let repo_filter = Term::from_field_text(indexer.source.repo_ref, &repo_ref.to_string());
        let terms = trigrams(hovered_text)
            .map(|token| Term::from_field_text(indexer.source.content, token.as_str()))
            .map(|term| {
                Box::new(TermQuery::new(term, IndexRecordOption::Basic))
                    as Box<dyn tantivy::query::Query>
            })
            .chain(std::iter::once(
                Box::new(TermQuery::new(repo_filter, IndexRecordOption::Basic))
                    as Box<dyn tantivy::query::Query>,
            ))
            // .chain(std::iter::once(Box::new(BooleanQuery::union(
            //     associated_langs
            //         .iter()
            //         .map(|l| {
            //             Term::from_field_bytes(
            //                 indexer.source.lang,
            //                 l.to_ascii_lowercase().as_bytes(),
            //             )
            //         })
            //         .map(|l| {
            //             Box::new(TermQuery::new(l, IndexRecordOption::Basic))
            //                 as Box<dyn tantivy::query::Query>
            //         })
            //         .collect::<Vec<_>>(),
            // ))
            //     as Box<dyn tantivy::query::Query>))
            .collect::<Vec<Box<dyn tantivy::query::Query>>>();

        BooleanQuery::intersection(terms)
    };
    let collector = TopDocs::with_limit(500);
    let searcher = indexes.file.reader.searcher();
    let results = searcher
        .search(&query, &collector)
        .expect("failed to search index");

    // if the hovered token is a def, ignore all other search-based defs
    let ignore_defs = {
        source_document
            .symbol_locations
            .scope_graph()
            .and_then(|graph| {
                graph
                    .node_by_range(text_range.start_byte(), text_range.end_byte())
                    .map(|idx| matches!(graph.graph[idx], NodeKind::Def(_)))
            })
            .unwrap_or_default()
    };

    let data = results
        .into_iter()
        .map(|result| (result, language_parsing.clone()))
        .filter_map(|((_, doc_addr), language_parsing)| {
            let retrieved_doc = searcher
                .doc(doc_addr)
                .expect("failed to get document by address");
            let doc = ContentDocument::read_document(file_source, retrieved_doc);
            // TODO(skcd): Fix the hoverable ranges logic because we need the language
            // etc to match up properly
            let hoverable_ranges = doc.hoverable_ranges(language, language_parsing)?;
            let data = target
                .find_iter(&doc.content)
                .map(|m| Range::from_byte_range(m.range(), &doc.line_end_indices))
                .filter(|range| hoverable_ranges.iter().any(|r| r.contains(range)))
                // why is this line below important??
                .filter(|range| {
                    !(text_range.start_byte() >= range.start_byte()
                        && text_range.end_byte() <= range.end_byte())
                })
                .map(|range| {
                    let start_byte = range.start_byte();
                    let end_byte = range.end_byte();
                    let is_def = doc
                        .symbol_locations
                        .scope_graph()
                        .and_then(|graph| {
                            graph
                                .node_by_range(start_byte, end_byte)
                                .map(|idx| matches!(graph.graph[idx], NodeKind::Def(_)))
                        })
                        .map(|d| {
                            if d {
                                OccurrenceKind::Definition
                            } else {
                                OccurrenceKind::Reference
                            }
                        })
                        .unwrap_or_default();
                    let highlight = start_byte..end_byte;
                    // TODO(skcd): Fix the snipper logic here, we can just make this
                    // easier
                    let snippet = Snipper::default()
                        .expand(highlight, &doc.content, &doc.line_end_indices)
                        .reify(&doc.content, &[]);

                    Occurrence {
                        kind: is_def,
                        range,
                        snippet,
                    }
                })
                .filter(|o| !(ignore_defs && o.is_definition())) // if ignore_defs is true & o is a def, omit it
                .collect::<Vec<_>>();

            let file = doc.relative_path;

            data.is_empty().not().then(|| FileSymbols {
                file: file.clone(),
                data,
            })
        })
        .collect::<Vec<_>>();

    Ok(data)
}

pub async fn refdef_runtime(
    repo_ref: &RepoRef,
    hovered_text: &str,
    text_range: &Range,
    relative_path: &str,
    content: &[u8],
    language: &str,
    language_parsing: Arc<TSLanguageParsing>,
) -> anyhow::Result<Vec<FileSymbols>> {
    let source_document = ContentDocument::build_document(
        repo_ref,
        content,
        language,
        relative_path,
        language_parsing.clone(),
    );
    // produce search based results here
    let regex_str = regex::escape(hovered_text);
    let target = regex::Regex::new(&format!(r"\b{regex_str}\b")).expect("failed to build regex");
    // perform a text search for hovered_text
    // let file_source = &indexes.file.source;
    // let indexer = &indexes.file;
    // let query = {
    //     let repo_filter = Term::from_field_text(indexer.source.repo_ref, &repo_ref.to_string());
    //     let terms = trigrams(hovered_text)
    //         .map(|token| Term::from_field_text(indexer.source.content, token.as_str()))
    //         .map(|term| {
    //             Box::new(TermQuery::new(term, IndexRecordOption::Basic))
    //                 as Box<dyn tantivy::query::Query>
    //         })
    //         .chain(std::iter::once(
    //             Box::new(TermQuery::new(repo_filter, IndexRecordOption::Basic))
    //                 as Box<dyn tantivy::query::Query>,
    //         ))
    //         // .chain(std::iter::once(Box::new(BooleanQuery::union(
    //         //     associated_langs
    //         //         .iter()
    //         //         .map(|l| {
    //         //             Term::from_field_bytes(
    //         //                 indexer.source.lang,
    //         //                 l.to_ascii_lowercase().as_bytes(),
    //         //             )
    //         //         })
    //         //         .map(|l| {
    //         //             Box::new(TermQuery::new(l, IndexRecordOption::Basic))
    //         //                 as Box<dyn tantivy::query::Query>
    //         //         })
    //         //         .collect::<Vec<_>>(),
    //         // ))
    //         //     as Box<dyn tantivy::query::Query>))
    //         .collect::<Vec<Box<dyn tantivy::query::Query>>>();

    //     BooleanQuery::intersection(terms)
    // };
    // let collector = TopDocs::with_limit(500);
    // let searcher = indexes.file.reader.searcher();
    // let results = searcher
    //     .search(&query, &collector)
    //     .expect("failed to search index");

    // if the hovered token is a def, ignore all other search-based defs
    let ignore_defs = {
        source_document
            .symbol_locations
            .scope_graph()
            .and_then(|graph| {
                graph
                    .node_by_range(text_range.start_byte(), text_range.end_byte())
                    .map(|idx| matches!(graph.graph[idx], NodeKind::Def(_)))
            })
            .unwrap_or_default()
    };

    let results = vec![source_document];

    let data = results
        .into_iter()
        .map(|result| (result, language_parsing.clone()))
        .filter_map(|(doc, language_parsing)| {
            // let retrieved_doc = searcher
            //     .doc(doc_addr)
            //     .expect("failed to get document by address");
            // let doc = ContentDocument::read_document(file_source, retrieved_doc);
            // TODO(skcd): Fix the hoverable ranges logic because we need the language
            // etc to match up properly
            let hoverable_ranges = doc.hoverable_ranges(language, language_parsing.clone())?;
            let data = target
                .find_iter(&doc.content)
                .map(|m| Range::from_byte_range(m.range(), &doc.line_end_indices))
                .filter(|range| hoverable_ranges.iter().any(|r| r.contains(range)))
                // why is this line below important??
                .filter(|range| {
                    !(text_range.start_byte() >= range.start_byte()
                        && text_range.end_byte() <= range.end_byte())
                })
                .map(|mut range| {
                    let start_byte = range.start_byte();
                    let end_byte = range.end_byte();
                    let (is_def, local_range) = doc
                        .symbol_locations
                        .scope_graph()
                        .map(|graph| {
                            let is_definition = graph
                                .node_by_range(start_byte, end_byte)
                                .map(|idx| matches!(graph.graph[idx], NodeKind::Def(_)))
                                .unwrap_or_default();
                            let full_range = graph.node_by_range(start_byte, end_byte).map(|idx| {
                                let node = &graph.graph[idx];
                                match node {
                                    NodeKind::Def(local_scope) => {
                                        Some(local_scope.local_scope.range)
                                    }
                                    _ => None,
                                }
                            });
                            (is_definition, full_range.flatten())
                        })
                        .map(|(d, range)| {
                            if d {
                                (OccurrenceKind::Definition, range)
                            } else {
                                (OccurrenceKind::Reference, range)
                            }
                        })
                        .unwrap_or_default();
                    if let Some(local_range) = local_range {
                        range = local_range;
                    }
                    let symbols: Vec<_> = doc
                        .symbol_locations
                        .scope_graph()
                        .map(|graph| graph.symbols(language_parsing.clone()))
                        .unwrap_or_default();
                    let highlight = start_byte..end_byte;
                    // TODO(skcd): Fix the snipper logic here, we can just make this
                    // easier
                    let snippet = Snipper::default()
                        .expand(highlight, &doc.content, &doc.line_end_indices)
                        .reify(&doc.content, symbols.as_slice());

                    Occurrence {
                        kind: is_def,
                        range,
                        snippet,
                    }
                })
                .filter(|o| !(ignore_defs && o.is_definition())) // if ignore_defs is true & o is a def, omit it
                .collect::<Vec<_>>();

            let file = doc.relative_path;

            data.is_empty().not().then(|| FileSymbols {
                file: file.clone(),
                data,
            })
        })
        .collect::<Vec<_>>();

    Ok(data)
}
