use std::{sync::Arc, time::Instant};

use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use async_trait::async_trait;

use crate::agentic::{symbol::identifier::LLMProperties, tool::kw_search::types::KeywordsReply};

use super::{
    tool::{
        KeywordSearch, KeywordSearchQuery, KeywordSearchQueryError, KeywordSearchQueryResponse,
    },
    types::KeywordsReplyError,
};

pub struct GoogleStudioKeywordSearch {
    llm_client: Arc<LLMBroker>,
    _fail_over_llm: LLMProperties,
}

impl GoogleStudioKeywordSearch {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            _fail_over_llm: fail_over_llm,
        }
    }

    pub fn system_message_for_keyword_search(&self, request: &KeywordSearchQuery) -> String {
        todo!()
    }

    pub fn user_message_for_keyword_search(&self, request: &KeywordSearchQuery) -> String {
        todo!()
    }
}

#[async_trait]
impl KeywordSearch for GoogleStudioKeywordSearch {
    async fn get_keywords(
        &self,
        request: KeywordSearchQuery,
    ) -> Result<KeywordsReply, KeywordsReplyError> {
        let root_request_id = request.root_request_id().to_owned();
        let model = request.llm().clone();
        let provider = request.provider().clone();
        let api_keys = request.api_keys().clone();

        let system_message =
            LLMClientMessage::system(self.system_message_for_keyword_search(&request));
        let user_message = LLMClientMessage::user(self.user_message_for_keyword_search(&request));
        let messages = LLMClientCompletionRequest::new(
            model,
            vec![system_message.clone(), user_message.clone()],
            0.2,
            None,
        );
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        let start = Instant::now();

        let response = self
            .llm_client
            .stream_completion(
                api_keys,
                messages,
                provider,
                vec![
                    ("event_type".to_owned(), "keyword_search".to_owned()),
                    ("root_id".to_owned(), root_request_id),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await?;

        println!("Keyword search response time: {:?}", start.elapsed());

        let parsed_response = KeywordsReply::parse_response(&response);

        match parsed_response {
            // Ok(parsed_response) => Ok(parsed_response.to_keyword_search_response()),
            Ok(parsed_response) => Ok(parsed_response),
            Err(e) => Err(e),
        }
    }
}
