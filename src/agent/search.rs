use crate::agent::{
    llm_funcs::llm::{self, Message},
    prompts,
    types::CodeSpan,
};

/// Here we allow the agent to perform search and answer workflow related questions
/// we will later use this for code planning and also code editing
use super::types::Agent;

use anyhow::Result;
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
            .stream_response(llm::OpenAIModel::GPT3_5_16k, prompt, vec![], 0.0)
            .await?;

        debug!("hyde response");

        let documents = prompts::try_parse_hypothetical_documents(&response);

        for doc in documents.iter() {
            info!(?doc, "hyde generated snippet");
        }

        Ok(documents)
    }
}
