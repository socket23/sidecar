use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};
use std::{sync::Arc, time::Instant};

use crate::agentic::{symbol::identifier::LLMProperties, tool::search::agentic::SearchPlanContext};

use super::agentic::{
    GenerateSearchPlan, GenerateSearchPlanError, SearchPlanQuery, SearchPlanResponse,
};

struct GoogleStudioPlanGenerator {
    llm_client: Arc<LLMBroker>,
    _fail_over_llm: LLMProperties,
}

impl GoogleStudioPlanGenerator {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            _fail_over_llm: fail_over_llm,
        }
    }

    // todo(zi): add CoT to system
    fn system_message_for_keyword_search(&self, request: &SearchPlanQuery) -> String {
        format!(r#"You will generate a search plan based on the provided context and user_query."#)
    }

    fn user_message_for_keyword_search(&self, request: &SearchPlanQuery) -> String {
        let context = request
            .context()
            .iter()
            .map(|c| match c {
                SearchPlanContext::RepoTree(repo_tree) => format!("RepoTree:\n{}", repo_tree),
            })
            .collect::<Vec<String>>()
            .join("\n");

        format!(
            r#"User query: {}\nContext: {:?}"#,
            request.user_query(),
            context,
        )
    }
}

#[async_trait]
impl GenerateSearchPlan for GoogleStudioPlanGenerator {
    async fn generate_search_plan(
        &self,
        request: &SearchPlanQuery,
    ) -> Result<SearchPlanResponse, GenerateSearchPlanError> {
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

        println!("Keyword search response: {:?}", response);

        todo!();
    }
}
