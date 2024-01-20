use std::{
    cmp::{max, min},
    collections::HashMap,
    sync::Arc,
};

use futures::stream;
use futures::StreamExt;
use llm_client::{
    broker::LLMBroker, clients::types::LLMType, provider::LLMProviderAPIKeys,
    tokenizer::tokenizer::LLMTokenizer,
};

use super::{
    mistral::MistralReRank,
    openai::OpenAIReRank,
    types::{
        CodeSpan, CodeSpanDigest, ReRankCodeSpan, ReRankCodeSpanError, ReRankCodeSpanRequest,
        ReRankCodeSpanResponse, ReRankStrategy,
    },
};

const SLIDING_WINDOW: usize = 10;
const TOP_K: usize = 5;

pub struct ReRankBroker {
    rerankers: HashMap<LLMType, Box<dyn ReRankCodeSpan + Send + Sync>>,
}

impl ReRankBroker {
    pub fn new() -> Self {
        let mut rerankers: HashMap<LLMType, Box<dyn ReRankCodeSpan + Send + Sync>> = HashMap::new();
        rerankers.insert(LLMType::GPT3_5_16k, Box::new(OpenAIReRank::new()));
        rerankers.insert(LLMType::Gpt4, Box::new(OpenAIReRank::new()));
        rerankers.insert(LLMType::Gpt4_32k, Box::new(OpenAIReRank::new()));
        rerankers.insert(LLMType::MistralInstruct, Box::new(MistralReRank::new()));
        rerankers.insert(LLMType::Mixtral, Box::new(MistralReRank::new()));
        Self { rerankers }
    }

    pub fn rerank_prompt(
        &self,
        request: ReRankCodeSpanRequest,
    ) -> Result<ReRankCodeSpanResponse, ReRankCodeSpanError> {
        let reranker = self.rerankers.get(&request.llm_type()).unwrap();
        reranker.rerank_prompt(request)
    }

    fn measure_tokens(
        &self,
        llm_type: &LLMType,
        code_digests: &[CodeSpanDigest],
        tokenizer: Arc<LLMTokenizer>,
    ) -> Result<usize, ReRankCodeSpanError> {
        let total_tokens: usize = code_digests
            .into_iter()
            .map(|code_digest| {
                let file_path = code_digest.file_path();
                let data = code_digest.data();
                let prompt = format!(
                    r#"FILEPATH: {file_path}
```
{data}
```"#
                );
                tokenizer.count_tokens_using_tokenizer(llm_type, &prompt)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum();
        Ok(total_tokens)
    }

    fn order_code_digests_listwise(
        &self,
        response: String,
        code_digests: Vec<CodeSpanDigest>,
    ) -> Vec<CodeSpanDigest> {
        let split_parts: Vec<String> = response
            .lines()
            .into_iter()
            .map(|line| line.to_owned())
            .collect();
        // We have the ordering from the split parts so we use that to get the
        // order
        let mut code_digests: HashMap<String, CodeSpanDigest> = code_digests
            .into_iter()
            .map(|code_digest| (code_digest.hash().to_owned(), code_digest))
            .collect();

        // Now create the ordered list by removing elements from the hashmap
        // and adding them to the vector, if any element is not found then
        // remove them from the list
        let mut final_list = vec![];
        for part in split_parts {
            if let Some(code_digest) = code_digests.remove(&part) {
                final_list.push(code_digest);
            }
        }
        // There might be elements which are not found in the list (LLM hallucinations)
        // we just iterate over the remaining elements and add them to the list
        for (_, code_digest) in code_digests {
            final_list.push(code_digest);
        }
        // Now reverse the final_list so the most relevant code is moved to the top
        final_list.reverse();
        final_list
    }

    pub async fn listwise_reranking(
        &self,
        api_keys: LLMProviderAPIKeys,
        request: ReRankCodeSpanRequest,
        client_broker: Arc<LLMBroker>,
        tokenizer: Arc<LLMTokenizer>,
    ) -> Result<Vec<CodeSpan>, ReRankCodeSpanError> {
        // We are given a list of code spans, we are going to do the following:
        // - implement a sliding window algorithm which goes over the snippets
        // and keeps ranking them until we have the list of top k snippets
        let code_spans = request.code_spans().to_vec();
        let mut digests = CodeSpan::to_digests(code_spans);
        // First we check if we need to do a sliding window here by measuring
        // against the token limit we have
        if request.token_limit()
            >= self.measure_tokens(request.llm_type(), &digests, tokenizer)? as i64
        {
            return Ok(digests
                .into_iter()
                .map(|digest| digest.get_code_span())
                .collect());
        }
        let mut end_index = min(SLIDING_WINDOW, digests.len()) - 1;
        while end_index < digests.len() {
            // Now that we are in the window, we have to take the elements from
            // (end_index - SLIDING_WINDOW)::(end_index)
            // and rank them, once we have these ranked
            // we move our window forward by TOP_K and repeat the process
            let code_spans = digests[max(end_index - SLIDING_WINDOW, 0)..=end_index]
                .iter()
                .map(|digest| digest.clone().get_code_span())
                .collect::<Vec<_>>();
            let request = ReRankCodeSpanRequest::new(
                request.user_query().to_owned(),
                request.limit(),
                request.token_limit(),
                code_spans,
                request.strategy().clone(),
                request.llm_type().clone(),
            );
            let prompt = self.rerank_prompt(request)?;
            if let ReRankCodeSpanResponse::ListWise(listwise_request) = prompt {
                let prompt = listwise_request.prompt;
                let code_span_digests = listwise_request.code_span_digests;
                let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
                let response = client_broker
                    .stream_answer(
                        api_keys.clone(),
                        prompt,
                        vec![("event_type".to_owned(), "listwise_reranking".to_owned())]
                            .into_iter()
                            .collect(),
                        sender,
                    )
                    .await?;

                // We have the updated list
                let updated_list = self.order_code_digests_listwise(response, code_span_digests);
                // Now we will in place replace the code spans from the digests from our start position
                // with the elements in this list
                for (index, code_span_digest) in updated_list.into_iter().enumerate() {
                    digests[end_index - SLIDING_WINDOW + index] = code_span_digest;
                }

                // Now move the window forward
                end_index += TOP_K;
            } else {
                return Err(ReRankCodeSpanError::WrongReRankStrategy);
            }
            // let response = client_broker.stream_completion(api_key, request, metadata, sender)
        }

        // At the end of this iteration we have our updated list of answers

        // First reverse the list so its ordered from the most relevant to the least
        digests.reverse();
        // Only take the request.limit() number of answers
        digests.truncate(request.limit());
        // convert back to the code span
        Ok(digests
            .into_iter()
            .map(|digest| digest.get_code_span())
            .collect())
    }

    pub async fn pointwise_reranking(
        &self,
        api_keys: LLMProviderAPIKeys,
        request: ReRankCodeSpanRequest,
        client_broker: Arc<LLMBroker>,
        tokenizer: Arc<LLMTokenizer>,
    ) -> Result<Vec<CodeSpan>, ReRankCodeSpanError> {
        // This approach uses the logits generated for yes and no to get the final
        // answer, since we are not use if we can logits yet on various platforms
        // we assume 1.0 for yes if thats the case or 0.0 for no otherwise
        let code_spans = request.code_spans().to_vec();
        let digests = CodeSpan::to_digests(code_spans);
        let answer_snippets = request.limit();

        // We first measure if we are within the token limit
        if request.token_limit()
            >= self.measure_tokens(request.llm_type(), &digests, tokenizer)? as i64
        {
            return Ok(digests
                .into_iter()
                .map(|digest| digest.get_code_span())
                .collect());
        }

        let request = ReRankCodeSpanRequest::new(
            request.user_query().to_owned(),
            request.limit(),
            request.token_limit(),
            digests
                .into_iter()
                .map(|digest| digest.get_code_span())
                .collect(),
            request.strategy().clone(),
            request.llm_type().clone(),
        );

        let prompt = self.rerank_prompt(request)?;

        if let ReRankCodeSpanResponse::PointWise(pointwise_prompts) = prompt {
            let response_with_code_digests = stream::iter(pointwise_prompts.into_iter())
                .map(|pointwise_prompt| async {
                    let prompt = pointwise_prompt.prompt;
                    let code_digest = pointwise_prompt.code_span_digest;
                    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
                    client_broker
                        .stream_answer(
                            api_keys.clone(),
                            prompt,
                            vec![("event_type".to_owned(), "pointwise_reranking".to_owned())]
                                .into_iter()
                                .collect(),
                            sender,
                        )
                        .await
                        .map(|response| (response, code_digest))
                })
                .buffer_unordered(25)
                .filter_map(|response| {
                    if let Ok((response, code_digest)) = response {
                        if response.trim().to_lowercase() == "yes" {
                            futures::future::ready(Some(code_digest))
                        } else {
                            futures::future::ready(None)
                        }
                    } else {
                        futures::future::ready(None)
                    }
                })
                .collect::<Vec<_>>()
                .await;
            // Now we only keep the code spans from the start until the length
            // of the limit we have
            let mut response_with_code_digests = response_with_code_digests
                .into_iter()
                .map(|code_digest| code_digest.get_code_span())
                .collect::<Vec<_>>();
            // Only keep until the answer snippets which are limited in this case
            response_with_code_digests.truncate(answer_snippets);
            return Ok(response_with_code_digests);
        } else {
            return Err(ReRankCodeSpanError::WrongReRankStrategy);
        }
    }

    pub async fn rerank(
        &self,
        api_keys: LLMProviderAPIKeys,
        request: ReRankCodeSpanRequest,
        // we need the broker here to get the right client
        client_broker: Arc<LLMBroker>,
        // we need the tokenizer here to count the tokens properly
        tokenizer_broker: Arc<LLMTokenizer>,
    ) -> Result<Vec<CodeSpan>, ReRankCodeSpanError> {
        let strategy = request.strategy();
        match strategy {
            ReRankStrategy::ListWise => {
                self.listwise_reranking(api_keys, request, client_broker, tokenizer_broker)
                    .await
            }
            ReRankStrategy::PointWise => {
                // We need to generate the prompt for this
                self.pointwise_reranking(api_keys, request, client_broker, tokenizer_broker)
                    .await
            }
        }
    }
}
