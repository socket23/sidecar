use crate::agent::{
    llm_funcs::{
        self,
        llm::{self, Message},
    },
    prompts,
    types::{AgentStep, CodeSpan},
};

/// Here we allow the agent to perform search and answer workflow related questions
/// we will later use this for code planning and also code editing
use super::types::Agent;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use futures::StreamExt;
use tiktoken_rs::CoreBPE;
use tracing::{debug, info};

use std::collections::HashSet;

const PATH_LIMIT: u64 = 30;
const PATH_LIMIT_USIZE: usize = 30;

impl Agent {
    pub async fn path_search(&mut self, query: &str) -> Result<String> {
        // Here we first take the user query and perform a lexical search
        // on all the paths which are present
        let mut path_matches = self
            .application
            .indexes
            .file
            .fuzzy_path_match(self.reporef(), query, PATH_LIMIT_USIZE)
            .await
            .map(|c| c.relative_path)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        // Now we try semantic search on the same query
        if self.application.semantic_client.is_some() && path_matches.is_empty() {
            path_matches = self
                .application
                .semantic_client
                .as_ref()
                .expect("is_some to hold above")
                .search(query, self.reporef(), PATH_LIMIT, 0, 0.0, true)
                .await?
                .into_iter()
                .map(|payload| payload.relative_path)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
        }

        // This also updates the path in the last exchange which has happened
        // with the agent
        let mut paths = path_matches
            .iter()
            .map(|p| (self.get_path_alias(p), p.to_string()))
            .collect::<Vec<_>>();
        paths.sort_by(|a: &(usize, String), b| a.0.cmp(&b.0));

        let response = paths
            .iter()
            .map(|(alias, path)| format!("{}: {}", alias, path))
            .collect::<Vec<_>>()
            .join("\n");

        // Now we want to update the path in agent
        let last_exchange = self.get_last_conversation_message();
        last_exchange.add_agent_step(super::types::AgentStep::Path {
            query: query.to_owned(),
            response: response.to_owned(),
        });

        Ok(response)
    }

    pub async fn code_search(&mut self, query: &str) -> Result<String> {
        const CODE_SEARCH_LIMIT: u64 = 10;
        // If we don't have semantic client we skip this one
        if self.application.semantic_client.is_none() {
            return Ok("".to_string());
        }
        let mut results = self
            .application
            .semantic_client
            .as_ref()
            .expect("is_none to hold")
            .search(query, self.reporef(), CODE_SEARCH_LIMIT, 0, 0.0, true)
            .await?;
        let hyde_snippets = self.hyde(query).await?;
        if !hyde_snippets.is_empty() {
            let hyde_snippet = hyde_snippets.first().unwrap();
            let hyde_search = self
                .application
                .semantic_client
                .as_ref()
                .expect("is_none to hold")
                .search(
                    hyde_snippet,
                    self.reporef(),
                    CODE_SEARCH_LIMIT,
                    0,
                    0.3,
                    true,
                )
                .await?;
            results.extend(hyde_search);
        }

        let mut code_snippets = results
            .into_iter()
            .map(|result| {
                let path_alias = self.get_path_alias(&result.relative_path);
                // convert it to a code snippet here
                let code_span = CodeSpan::new(
                    result.relative_path,
                    path_alias,
                    result.start_line,
                    result.end_line,
                    result.text,
                );
                code_span
            })
            .collect::<Vec<_>>();

        code_snippets.sort_by(|a, b| a.alias.cmp(&b.alias).then(a.start_line.cmp(&b.start_line)));

        for code_chunk in code_snippets
            .iter()
            .filter(|code_snippet| !code_snippet.is_empty())
        {
            // Update the last conversation context with the code snippets which
            // we got here
            let last_exchange = self.get_last_conversation_message();
            last_exchange.add_code_spans(code_chunk.clone());
        }

        let response = code_snippets
            .iter()
            .filter(|c| !c.is_empty())
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("\n\n");

        // Now we want to also update the step of the exchange to highlight that
        // we did a search here
        let last_exchange = self.get_last_conversation_message();
        last_exchange.add_agent_step(super::types::AgentStep::Code {
            query: query.to_owned(),
            response: response.to_owned(),
        });

        // Now that we have done the code search, we need to figure out what we
        // can do next with all the snippets, some ideas here include dedup and
        // also to join snippets together
        Ok(response)
    }

    async fn hyde(&self, query: &str) -> Result<Vec<String>> {
        let prompt = vec![Message::system(&prompts::hypothetical_document_prompt(
            query,
        ))];

        let response = self
            .get_llm_client()
            .stream_response(llm::OpenAIModel::GPT3_5_16k, prompt, vec![], 0.0, None)
            .await?;

        debug!("hyde response");

        let documents = prompts::try_parse_hypothetical_documents(&response);

        for doc in documents.iter() {
            info!(?doc, "hyde generated snippet");
        }

        Ok(documents)
    }

    pub async fn process_files(&mut self, query: &str, path_aliases: &[usize]) -> Result<String> {
        const MAX_CHUNK_LINE_LENGTH: usize = 20;
        const CHUNK_MERGE_DISTANCE: usize = 10;
        const MAX_TOKENS: usize = 15400;

        let paths = path_aliases
            .iter()
            .copied()
            .map(|i| self.paths().nth(i).ok_or(i).map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|i| anyhow!("invalid path alias {i}"))?;

        debug!(?query, ?paths, "processing file");

        // Immutable reborrow of `self`, to copy freely to async closures.
        let self_ = &*self;
        let chunks = futures::stream::iter(paths.clone())
            .map(|path| async move {
                tracing::debug!(?path, "reading file");

                let lines = self_
                    .get_file_content(&path)
                    .await?
                    .with_context(|| format!("path does not exist in the index: {path}"))?
                    .content
                    .lines()
                    .enumerate()
                    .map(|(i, line)| format!("{} {line}", i + 1))
                    .collect::<Vec<_>>();

                let bpe = tiktoken_rs::get_bpe_from_model("gpt-3.5-turbo")?;

                let iter =
                    tokio::task::spawn_blocking(|| trim_lines_by_tokens(lines, bpe, MAX_TOKENS))
                        .await
                        .context("failed to split by token")?;

                Result::<_>::Ok((iter, path.clone()))
            })
            // Buffer file loading to load multiple paths at once
            .buffered(10)
            .map(|result| async {
                let (lines, path) = result?;

                // The unwraps here should never fail, we generated this string above to always
                // have the same format.
                let start_line = lines[0]
                    .split_once(' ')
                    .unwrap()
                    .0
                    .parse::<usize>()
                    .unwrap()
                    - 1;

                // We store the lines separately, so that we can reference them later to trim
                // this snippet by line number.
                let contents = lines.join("\n");
                let prompt = prompts::file_explanation(query, &path, &contents);

                debug!(?path, "asking GPT-3.5 to get traces from the file");

                let json = self
                    .get_llm_client()
                    .stream_response(
                        llm_funcs::llm::OpenAIModel::GPT3_5_16k,
                        vec![llm_funcs::llm::Message::system(&prompt)],
                        vec![],
                        0.0,
                        Some(0.2),
                    )
                    .await?;

                #[derive(
                    serde::Deserialize,
                    serde::Serialize,
                    PartialEq,
                    Eq,
                    PartialOrd,
                    Ord,
                    Copy,
                    Clone,
                    Debug,
                )]
                struct Range {
                    start: usize,
                    end: usize,
                }

                #[derive(serde::Serialize)]
                struct RelevantChunk {
                    #[serde(flatten)]
                    range: Range,
                    code: String,
                }

                let mut line_ranges: Vec<Range> = serde_json::from_str::<Vec<Range>>(&json)?
                    .into_iter()
                    .filter(|r| r.start > 0 && r.end > 0)
                    .map(|mut r| {
                        r.end = r.end.min(r.start + MAX_CHUNK_LINE_LENGTH); // Cap relevant chunk size by line number
                        r
                    })
                    .map(|r| Range {
                        start: r.start - 1,
                        end: r.end,
                    })
                    .collect();

                line_ranges.sort();
                line_ranges.dedup();

                let relevant_chunks = line_ranges
                    .into_iter()
                    .fold(Vec::<Range>::new(), |mut exps, next| {
                        if let Some(prev) = exps.last_mut() {
                            if prev.end + CHUNK_MERGE_DISTANCE >= next.start {
                                prev.end = next.end;
                                return exps;
                            }
                        }

                        exps.push(next);
                        exps
                    })
                    .into_iter()
                    .filter_map(|range| {
                        Some(RelevantChunk {
                            range,
                            code: lines
                                .get(
                                    range.start.saturating_sub(start_line)
                                        ..=range.end.saturating_sub(start_line),
                                )?
                                .iter()
                                .map(|line| line.split_once(' ').unwrap().1)
                                .collect::<Vec<_>>()
                                .join("\n"),
                        })
                    })
                    .collect::<Vec<_>>();

                Ok::<_, anyhow::Error>((relevant_chunks, path))
            });

        let processed = chunks
            .boxed()
            .buffered(5)
            .filter_map(|res| async { res.ok() })
            .collect::<Vec<_>>()
            .await;

        let mut chunks = processed
            .into_iter()
            .flat_map(|(relevant_chunks, path)| {
                let alias = self.get_path_alias(&path);

                relevant_chunks.into_iter().map(move |c| {
                    CodeSpan::new(
                        path.clone(),
                        alias,
                        c.range.start.try_into().unwrap(),
                        c.range.end.try_into().unwrap(),
                        c.code,
                    )
                })
            })
            .collect::<Vec<_>>();

        chunks.sort_by(|a, b| a.alias.cmp(&b.alias).then(a.start_line.cmp(&b.start_line)));

        for chunk in chunks.iter().filter(|c| !c.is_empty()) {
            let last_conversation_message = self.get_last_conversation_message();
            last_conversation_message.add_code_spans(chunk.clone());
        }

        let response = chunks
            .iter()
            .filter(|c| !c.is_empty())
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("\n\n");

        let last_exchange = self.get_last_conversation_message();
        last_exchange.add_agent_step(AgentStep::Proc {
            query: query.to_owned(),
            paths,
            response: response.to_owned(),
        });

        Ok(response)
    }
}

fn trim_lines_by_tokens(lines: Vec<String>, bpe: CoreBPE, max_tokens: usize) -> Vec<String> {
    let line_tokens = lines
        .iter()
        .map(|line| bpe.encode_ordinary(line).len())
        .collect::<Vec<_>>();

    let mut trimmed_lines = Vec::new();

    // Push lines to `trimmed_lines` until we reach the maximum number of tokens.
    let mut i = 0usize;
    let mut tokens = 0usize;
    while i < lines.len() && tokens < max_tokens {
        tokens += line_tokens[i];
        trimmed_lines.push(lines[i].clone());
        i += 1;
    }

    trimmed_lines
}
