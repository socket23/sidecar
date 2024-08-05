use std::sync::Arc;

use llm_client::broker::LLMBroker;

use async_trait::async_trait;

use crate::agentic::symbol::identifier::LLMProperties;

use super::tool::{
    KeywordSearch, KeywordSearchQuery, KeywordSearchQueryError, KeywordSearchQueryResponse,
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
}

#[async_trait]
impl KeywordSearch for GoogleStudioKeywordSearch {
    async fn get_keywords(
        &self,
        request: KeywordSearchQuery,
    ) -> Result<KeywordSearchQueryResponse, KeywordSearchQueryError> {
        todo!();
    }
}
