//! We are going to implement how tht agent is going to use user context

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

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
use crate::agent::types::CodeSpan;
use crate::application::application::Application;
use crate::chunking::languages::TSLanguageParsing;
use crate::indexes::schema::CodeSnippetTokenizer;
use crate::indexes::schema::QuickCodeSnippet;
use crate::indexes::schema::QuickCodeSnippetDocument;
use crate::webserver::agent::FileContentValue;
use crate::webserver::agent::VariableInformation;
use crate::webserver::agent::VariableType;

use super::llm_funcs::LlmClient;
use super::types::Agent;
use super::types::ExtendedVariableInformation;

impl Agent {
    pub async fn answer_context_from_user_data(
        &mut self,
        messages: Vec<llm_funcs::llm::Message>,
    ) -> anyhow::Result<()> {
        let bpe = tiktoken_rs::get_bpe_from_model(self.model.tokenizer)?;
        let query = query_from_messages(messages.as_slice());

        let history_token_limit = self.model.history_tokens_limit;
        let mut prompt_token_limit = self.model.prompt_tokens_limit;

        // we look at our messages and check how many more tokens we can save
        // and send over
        let history_tokens_in_use = tiktoken_rs::num_tokens_from_messages(
            self.model.tokenizer,
            messages
                .iter()
                .map(|message| message.into())
                .collect::<Vec<_>>()
                .as_slice(),
        )?;

        // we get additional breathing room if we are using less history tokens
        // than required
        prompt_token_limit = prompt_token_limit
            + std::cmp::max(
                0,
                (history_token_limit as i64) - (history_tokens_in_use as i64),
            ) as usize;
        let open_file_data = self
            .get_last_conversation_message_immutable()
            .get_active_window();
        let user_context = self
            .user_context
            .as_ref()
            .expect("user_context to be there")
            .clone();

        // What if the user_context is empty? we just use the current open window
        // as the context for the answer
        let code_spans = if user_context.is_empty() {
            match open_file_data {
                Some(open_file) => {
                    let file_tokens = bpe.encode_ordinary(&open_file.file_content).len();
                    if file_tokens <= prompt_token_limit {
                        // we are good, no need to filter things out here
                        vec![CodeSpan::from_active_window(open_file, 0)]
                    } else {
                        // we are not good, need to truncate here until we fit
                        // in the prompt limit
                        let files = vec![FileContentValue {
                            file_path: open_file.file_path.clone(),
                            file_content: open_file.file_content.clone(),
                            language: open_file.language.clone(),
                        }];
                        self.truncate_files_to_fit_in_limit(
                            files,
                            messages,
                            prompt_token_limit,
                            &bpe,
                        )
                        .await?
                    }
                }
                None => {
                    // we have none, so do nothing here
                    vec![]
                }
            }
        } else {
            // since we have some user context, this takes precedence over the open file in the editor
            let file_content = user_context.file_content_map;
            let user_variables = user_context.variables;
            // We will handle the selection variables first here since those are super important
            let (selection_variables, remaining_tokens) = self
                .select_user_variable_selection_to_fit_in_context(
                    user_variables,
                    file_content.clone(),
                    prompt_token_limit,
                    &bpe,
                )
                .await?;
            // Save this to the last message
            self.save_extended_code_selection_variables(selection_variables)?;
            self.truncate_files_to_fit_in_limit(file_content, messages, remaining_tokens, &bpe)
                .await?
        };
        // Now we update the code spans which we have selected
        let _ = self.save_code_snippets_response(&query, code_spans);
        // We also retroactively save the last conversation to the database
        if let Some(last_conversation) = self.conversation_messages.last() {
            // save the conversation to the DB
            let _ = last_conversation
                .save_to_db(self.sql_db.clone(), self.reporef().clone())
                .await;
            // send it over the sender
            let _ = self.sender.send(last_conversation.clone()).await;
        }
        Ok(())
    }

    async fn select_user_variable_selection_to_fit_in_context(
        &mut self,
        user_variables: Vec<VariableInformation>,
        file_content_map: Vec<FileContentValue>,
        prompt_token_limit: usize,
        bpe: &tiktoken_rs::CoreBPE,
        // We return here the extended variable information and the tokens which remain
    ) -> anyhow::Result<(Vec<ExtendedVariableInformation>, usize)> {
        // Here we will try to get the enclosing span of the context which the user has selected
        // and try to expand on it?
        let mut tokens_used = 0;
        let selection_variables = user_variables
            .into_iter()
            .filter(|variable| variable.is_selection())
            .filter(|variable| {
                file_content_map
                    .iter()
                    .any(|file| file.file_path == variable.fs_file_path)
            })
            .collect::<Vec<_>>();

        let extended_variables = selection_variables
            .into_iter()
            .filter_map(|selection_variable| {
                // These are 1 indexed in our case, so we have to work with that
                let start_line = selection_variable.start_position.line;
                let end_line = selection_variable.end_position.line;
                let file_path = selection_variable.fs_file_path.clone();
                let file_content = file_content_map
                    .iter()
                    .find(|file| file.file_path == file_path)
                    .unwrap()
                    .file_content
                    .clone();
                let file_path_alias = self
                    .get_last_conversation_message_immutable()
                    .get_file_path_alias(&file_path);
                if file_path_alias.is_none() {
                    return None;
                }
                let file_lines = file_content.split('\n').collect::<Vec<_>>();
                let start_line = start_line as usize;
                let end_line = end_line as usize;
                let content = file_lines[start_line..end_line].join("\n");
                let tokens = bpe.encode_ordinary(&content).len();
                if tokens_used + tokens >= prompt_token_limit {
                    return None;
                } else {
                    tokens_used += tokens;
                }
                Some(ExtendedVariableInformation::new(
                    selection_variable.to_agent_type(),
                    Some(CodeSpan::new(
                        file_path,
                        file_path_alias.expect("is_none check above to work"),
                        start_line as u64,
                        end_line as u64,
                        content.to_owned(),
                        Some(1.0),
                    )),
                    content,
                ))
            })
            .collect::<Vec<_>>();

        // Now we can safely try and expand the selection context here if required, atleast we can get the function beginning and the end using this or the class
        // start and end here if required
        Ok((extended_variables, prompt_token_limit - tokens_used))
    }

    async fn truncate_files_to_fit_in_limit(
        &mut self,
        files: Vec<FileContentValue>,
        messages: Vec<llm_funcs::llm::Message>,
        prompt_token_limit: usize,
        bpe: &tiktoken_rs::CoreBPE,
    ) -> anyhow::Result<Vec<CodeSpan>> {
        let query = query_from_messages(messages.as_slice());
        let previous_message = messages;
        // we always capture the user variables as much as possible since these
        // are important and have been provided by the user
        let file_path_to_index: HashMap<String, usize> = files
            .iter()
            .enumerate()
            .map(|(idx, file_value)| (file_value.file_path.clone(), idx))
            .collect();

        let self_ = &*self;
        let lexical_search_and_file = stream::iter(
            files
                .into_iter()
                .map(|user_file| (user_file, previous_message.to_vec())),
        )
        .map(|(file_value, previous_message)| async move {
            let language = &file_value.language;
            // First generate the outline for the file here
            let language_config = self_.application.language_parsing.for_lang(language);
            let file_content = &file_value.file_content;
            let fs_file_path = &file_value.file_path;
            // Now we can query gpt3.5 with the output here and aks it to
            // generate the code search keywords which we need
            let system_prompt = prompts::proc_search_system_prompt(
                language_config.map(|lang_config| {
                    lang_config.generate_file_outline_str(file_content.as_bytes())
                }),
                &file_value.file_path,
            );
            let functions = serde_json::from_value::<Vec<llm_funcs::llm::Function>>(
                prompts::proc_function_truncate(),
            )
            .unwrap();
            let messages = vec![llm_funcs::llm::Message::system(&system_prompt)]
                .into_iter()
                .chain(previous_message)
                .chain(vec![
                    llm_funcs::llm::Message::user(&format!(
                        "We are working on {fs_file_path} so choose your answer for this file."
                    )),
                    llm_funcs::llm::Message::user("CALL A FUNCTION!. Do not answer"),
                ])
                .collect::<Vec<_>>();
            let response = self_
                .get_llm_client()
                .stream_function_call(
                    llm_funcs::llm::OpenAIModel::GPT4_Turbo,
                    messages,
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
                    Ok(Some(AgentAction::Proc { query, paths: _ })) => {
                        return Some((query, file_value))
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
        .buffer_unordered(10)
        .filter_map(|value| async { value })
        .collect::<Vec<_>>()
        .await;

        let candidates =
            gather_code_snippets_for_answer(lexical_search_and_file, self.application.clone())
                .await
                .into_iter()
                .take(50)
                .collect::<Vec<_>>();
        let ranked_candidates = re_rank_code_snippets(
            &query,
            self.get_llm_client(),
            candidates,
            prompt_token_limit,
            bpe,
        )
        .await;
        let code_snippets = merge_consecutive_chunks(ranked_candidates)
            .into_iter()
            .map(|code_snippet| {
                let index = file_path_to_index
                    .get(code_snippet.path.as_str())
                    .expect("file path to be present")
                    .clone();
                CodeSpan::from_quick_code_snippet(code_snippet, index)
            })
            .collect::<Vec<_>>();
        Ok(code_snippets)
    }
}

/// Takes a slice of `llm_funcs::llm::Message` and returns a string containing the content of the messages from the user and assistant roles.
///
/// # Arguments
///
/// * `messages` - A slice of `llm_funcs::llm::Message` representing the messages to query.
///
/// # Returns
///
/// A string containing the content of the messages from the user and assistant roles, with each message separated by a newline character.
fn query_from_messages(messages: &[llm_funcs::llm::Message]) -> String {
    messages
        .iter()
        .map(|message| match message {
            llm_funcs::llm::Message::PlainText {
                role: llm_funcs::llm::Role::User,
                content,
            } => {
                format!("User: {}", content)
            }
            llm_funcs::llm::Message::PlainText {
                role: llm_funcs::llm::Role::Assistant,
                content,
            } => {
                format!("Assistant: {}", content)
            }
            _ => "".to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl VariableInformation {
    /// Converts the user context to a formatted prompt string.
    /// The prompt string includes the file path, start and end line numbers,
    /// language, and formatted content of the user context.
    ///
    /// # Returns
    ///
    /// A string representing the formatted prompt.
    pub fn to_prompt(&self) -> String {
        let file_path = &self.fs_file_path;
        let start_line = self.start_position.line;
        let end_line = self.end_position.line;
        let language = &self.language.to_lowercase();
        let formatted_content = self
            .content
            .split('\n')
            .enumerate()
            .into_iter()
            .map(|(idx, line)| format!("{}:{}", idx + start_line as usize, line))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"Location: {file_path}:{start_line}-{end_line}
```{language}
{formatted_content}
```
"#
        )
    }
}

async fn re_rank_code_snippets(
    query: &str,
    llm_client: Arc<LlmClient>,
    candidates: Vec<QuickCodeSnippetDocument>,
    prompt_token_limit: usize,
    encoder: &tiktoken_rs::CoreBPE,
) -> Vec<QuickCodeSnippetDocument> {
    let mut logprob_scored = stream::iter(
        candidates
            .into_iter()
            .map(|candidate| (candidate, llm_client.clone())),
    )
    .map(|(candidate, llm_client)| async move {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let completion_request = prompts::code_snippet_important(
            &candidate.unique_key(),
            &candidate.content,
            &candidate.language,
            query,
        );
        // we also send a logit-bias to the request, since we want to guard
        // against the model generating yes and no and only those values
        let answer = llm_client
            .stream_completion_call(
                llm_funcs::llm::OpenAIModel::GPT3_5Instruct,
                &completion_request,
                sender,
                Some(
                    // these are the yes and no tokens we get from the cl100k_base tokenizer
                    // for the gpt family of models
                    vec![("9642".to_owned(), 1.into()), ("2822".to_owned(), 1.into())]
                        .into_iter()
                        .collect(),
                ),
            )
            .await;
        let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
        let mut logprobs = 0.0;
        let mut total_tokens = 0.0;
        receiver_stream
            .for_each(|item| {
                let _ = item.logprobs.map(|logprob| {
                    logprob.into_iter().for_each(|prob| {
                        prob.and_then(|prob| {
                            logprobs += prob;
                            total_tokens = total_tokens + 1.0;
                            Some(())
                        });
                    });
                });
                futures::future::ready(())
            })
            .await;
        // Now we will calculate the average log probability score
        let average_logprobs = logprobs / total_tokens as f32;
        let answer = match answer {
            Ok(answer) => answer.to_lowercase().trim().to_owned(),
            Err(_) => "no".to_owned(),
        };
        if answer == "yes" {
            Some((average_logprobs, candidate))
        } else {
            None
        }
    })
    .buffer_unordered(25)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .filter_map(|s| s)
    .collect::<Vec<_>>();
    // We sort it in decreasing order of the logprob score
    logprob_scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let mut current_tokens_used = 0;
    let mut final_candidates = Vec::new();
    for (_, candidate) in logprob_scored.into_iter() {
        // This is not exactly correct as we are missing the tokens from the proper formatting of
        // the code snippet, but because we have some headroom this is okay
        current_tokens_used += encoder.encode_ordinary(&candidate.content).len();
        if current_tokens_used >= prompt_token_limit {
            break;
        } else {
            final_candidates.push(candidate);
        }
    }
    final_candidates
}

fn merge_consecutive_chunks(
    code_snippets: Vec<QuickCodeSnippetDocument>,
) -> Vec<QuickCodeSnippetDocument> {
    const CHUNK_MERGE_DISTANCE: usize = 0;
    let mut file_to_code_snippets: HashMap<String, Vec<QuickCodeSnippetDocument>> =
        Default::default();

    code_snippets.into_iter().for_each(|code_snippet| {
        let file_path = code_snippet.path.clone();
        let code_snippets = file_to_code_snippets
            .entry(file_path)
            .or_insert_with(Vec::new);
        code_snippets.push(code_snippet);
    });

    // We want to sort the code snippets in increasing order of the start line
    file_to_code_snippets
        .iter_mut()
        .for_each(|(_, code_snippets)| {
            code_snippets.sort_by(|a, b| a.start_line.cmp(&b.start_line));
        });

    // Now we will merge chunks which are in the range of CHUNK_MERGE_DISTANCE
    let results = file_to_code_snippets
        .into_iter()
        .map(|(file_path, mut code_snippets)| {
            let mut final_code_snippets = Vec::new();
            let mut current_code_snippet = code_snippets.remove(0);
            for code_snippet in code_snippets {
                if code_snippet.start_line - current_code_snippet.end_line
                    <= CHUNK_MERGE_DISTANCE as u64
                {
                    // We can merge these two code snippets
                    current_code_snippet.end_line = code_snippet.end_line;
                    current_code_snippet.content =
                        format!("{}{}", current_code_snippet.content, code_snippet.content);
                } else {
                    // We cannot merge these two code snippets
                    final_code_snippets.push(current_code_snippet);
                    current_code_snippet = code_snippet;
                }
            }
            final_code_snippets.push(current_code_snippet);
            final_code_snippets
                .into_iter()
                .map(|code_snippet| QuickCodeSnippetDocument {
                    path: file_path.clone(),
                    content: code_snippet.content,
                    start_line: code_snippet.start_line,
                    end_line: code_snippet.end_line,
                    score: code_snippet.score,
                    language: code_snippet.language,
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();
    results
}

async fn gather_code_snippets_for_answer(
    candidates: Vec<(String, FileContentValue)>,
    application: Application,
) -> Vec<QuickCodeSnippetDocument> {
    let mut quick_code_snippet_index = QuickCodeSnippetIndex::create_in_memory_index();
    quick_code_snippet_index = build_tantivy_index(
        quick_code_snippet_index,
        candidates
            .to_vec()
            .into_iter()
            .map(|(_, value)| value)
            .collect(),
        application.clone(),
    );

    // Now we need to perform the lexical and then the embedding search
    // we will do this in parallel
    let mut lexical_search_results: Vec<_> = relativize_scores_for_snippets(
        candidates
            .iter()
            .flat_map(|(query, file_content_value)| {
                dbg!(&query);
                quick_code_snippet_index.lexical_query(&file_content_value.file_path, query)
            })
            .collect::<Vec<_>>(),
    );

    let embedding_search_results: Vec<_> = relativize_scores_for_snippets(
        stream::iter(
            candidates
                .into_iter()
                .map(|candidate| (candidate, application.clone())),
        )
        .map(|(candidate, application)| {
            rank_spans_on_embeddings(
                candidate.1.file_path,
                candidate.1.file_content,
                candidate.0,
                application.clone(),
                candidate.1.language,
            )
        })
        .buffer_unordered(10)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect(),
    );

    let embedding_search_map = embedding_search_results
        .iter()
        .map(|embeddings| (embeddings.unique_key(), embeddings.score))
        .collect::<HashMap<_, _>>();

    // Now we want to merge the results together and get back the results

    let mut quick_code_snippet_set: HashSet<String> = HashSet::new();
    lexical_search_results = lexical_search_results
        .into_iter()
        .map(|mut lexical_search_result| {
            let mut final_result = lexical_search_result.score * 2.5;
            quick_code_snippet_set.insert(lexical_search_result.unique_key());
            if let Some(embedding_score) =
                embedding_search_map.get(&lexical_search_result.unique_key())
            {
                final_result += embedding_score;
            }
            lexical_search_result.score = final_result;
            lexical_search_result
        })
        .collect::<Vec<_>>();

    embedding_search_results
        .into_iter()
        .for_each(|embedding_search_result| {
            if !quick_code_snippet_set.contains(&embedding_search_result.unique_key()) {
                lexical_search_results.push(embedding_search_result);
            }
        });

    lexical_search_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    // This is the final lexical search results we get
    lexical_search_results
}

fn relativize_scores_for_snippets(
    mut quick_code_snippets: Vec<QuickCodeSnippetDocument>,
) -> Vec<QuickCodeSnippetDocument> {
    if quick_code_snippets.is_empty() {
        return quick_code_snippets;
    }
    // Here we will also reduce the score to be in a range from 0.5 -> 1 for the
    // lexical search
    let max_score = quick_code_snippets
        .iter()
        .map(|lexical_search_snippet| lexical_search_snippet.score)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let mut min_score = quick_code_snippets
        .iter()
        .map(|lexical_search_snippet| lexical_search_snippet.score)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    if min_score == max_score {
        min_score = 0.0;
    }
    // Now relativize the scores in this range
    quick_code_snippets
        .iter_mut()
        .for_each(|lexical_search_snippet| {
            lexical_search_snippet.score =
                (lexical_search_snippet.score - min_score) / (max_score - min_score) * 0.5 + 0.5;
        });
    quick_code_snippets
}

struct QuickCodeSnippetIndex {
    schema: QuickCodeSnippet,
    index: tantivy::Index,
    reader: tantivy::IndexReader,
    writer: tantivy::IndexWriter,
}

impl QuickCodeSnippetIndex {
    fn create_in_memory_index() -> Self {
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
                100_000_000 * std::thread::available_parallelism().unwrap().get(),
            )
            .expect("index writer to not fail");
        Self {
            schema: QuickCodeSnippet::new(),
            index,
            reader,
            writer,
        }
    }

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
            .map(|token| format!(r#"content:{}"#, token.text))
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
        self,
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
        dbg!(chunks.len());
        chunks
            .into_iter()
            .map(|chunk| {
                let data = chunk.data.unwrap_or_default();
                doc!(
                    schema.start_line => chunk.start as u64,
                    schema.end_line => chunk.end as u64,
                    schema.path => self.file_path.to_owned(),
                    schema.content =>  data.to_owned(),
                    schema.language => self.language.to_owned(),
                )
            })
            .collect::<Vec<_>>()
    }
}

// Now we want to build the index over here
fn build_tantivy_index(
    mut quick_code_snippet_index: QuickCodeSnippetIndex,
    file_content_value: Vec<FileContentValue>,
    application: Application,
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
    query: String,
    application: Application,
    language: String,
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
        .chunk_file(&fs_file_path, &file_content, None, Some(language.as_str()))
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
            language.clone(),
        ));
    }
    final_generation
}
