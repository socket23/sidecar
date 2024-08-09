use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys},
};
use serde_xml_rs::from_str;
use std::{sync::Arc, time::Instant};

use crate::{
    agent::search,
    agentic::{
        symbol::identifier::LLMProperties,
        tool::search::agentic::{SearchPlanContext, SerdeError},
    },
};

use super::{
    agentic::{GenerateSearchPlan, GenerateSearchPlanError, SearchPlanQuery, SearchPlanResponse},
    exp::{Context, IterativeSearchError, LLMOperations, SearchQuery, SearchRequests},
};

pub struct GoogleStudioLLM {
    model: LLMType,
    provider: LLMProvider,
    api_keys: LLMProviderAPIKeys,
    root_directory: String,
    root_request_id: String,
    client: Arc<LLMBroker>,
}

impl GoogleStudioLLM {
    pub fn new(root_directory: String, client: Arc<LLMBroker>, root_request_id: String) -> Self {
        Self {
            model: LLMType::GeminiProFlash,
            provider: LLMProvider::GoogleAIStudio,
            api_keys: LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
            )),
            root_directory,
            root_request_id,
            client,
        }
    }
    pub fn system_message_for_generate_search_query(&self, context: &Context) -> String {
        format!(
            r#"You are an autonomous AI assistant.
Your task is to locate the code relevant to an issue.

# Instructions:

1. Understand The Issue:
Read the <issue> tag to understand the issue.

2. Review Current File Context:
Examine the <file_context> tag to see which files and code spans have already been identified.
If you believe that all relevant files have been identified, you can finish the search by setting complete to true.

3. Consider the Necessary Search Parameters:
Determine if specific file types, directories, function or class names or code patterns are mentioned in the issue.
If you can you should always try to specify the search parameters as accurately as possible.
You can do more than one search request at the same time so you can try different search parameters to cover all possible relevant code.

4. Ensure At Least One Search Parameter:
Make sure that at least one of File or Keyword is provided. File allows you to search for file names. Keyword allows you to search for symbols such as class and function names.

5. Formulate the Search function:
For files, you do not need to provide the extension. For Keyword, use only uninterrupted strings, not phrases.

6. Execute the Search:
Execute the search by providing the search parameters and your thoughts on how to approach this task in XML. 

Think step by step and write out your thoughts in the thinking field.

Examples:

User:
The generate_report function sometimes produces incomplete reports under certain conditions. This function is part of the reporting module. Locate the generate_report function in the reports directory to debug and fix the issue.

Assistant:
<reply>
<search_requests>
<request>
<thinking>
</thinking>
<search_tool>Keyword</search_tool>
<query>
generate_report
</query>
</request>
<request>
<thinking>
</thinking>
<search_tool>File</search_tool>
<query>
report
</query>
</request>
</search_requests>
</reply>
"#
        )
    }

    pub fn user_message_for_generate_search_query(&self, context: &Context) -> String {
        format!(
            r#"<issue>{}</issue>
<file_context>{}</file_context>
        "#,
            context.user_query(),
            context.file_paths_as_strings().join(", ")
        )
    }

    // todo: remove llm_query
    pub async fn generate_search_queries(&self, context: Context) -> Vec<SearchQuery> {
        println!("googlestudioplangenerator::generate_search_plan");

        println!(
            "googlestudioplangenerator::generate_search_plan::context: \n{:?}",
            context
        );

        let system_message =
            LLMClientMessage::system(self.system_message_for_generate_search_query(&context));
        let user_message =
            LLMClientMessage::user(self.user_message_for_generate_search_query(&context));

        let messages = LLMClientCompletionRequest::new(
            self.model.to_owned(),
            vec![system_message.clone(), user_message.clone()],
            0.2,
            None,
        );

        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        let response = self
            .client
            .stream_completion(
                self.api_keys.to_owned(),
                messages,
                self.provider.to_owned(),
                vec![
                    (
                        "event_type".to_owned(),
                        "generate_search_tool_query".to_owned(),
                    ),
                    ("root_id".to_owned(), self.root_request_id.to_string()),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await;

        match response {
            Ok(response) => {
                println!("{response}");
                let _ = GoogleStudioLLM::parse_response(&response);
            }
            Err(err) => eprintln!("{:?}", err),
        }

        todo!();

        // parse response into SearchQuery

        // SearchQuery::new("some query".to_owned())
    }

    fn parse_response(response: &str) -> Result<SearchRequests, IterativeSearchError> {
        let lines = response
            .lines()
            .skip_while(|l| !l.contains("<reply>"))
            .skip(1)
            .take_while(|l| !l.contains("</reply>"))
            .collect::<Vec<&str>>()
            .join("\n");

        from_str::<SearchRequests>(&lines).map_err(|error| {
            eprintln!("{:?}", error);
            IterativeSearchError::SerdeError(SerdeError::new(error, lines))
        })
    }
}

#[async_trait]
impl LLMOperations for GoogleStudioLLM {
    async fn generate_search_query(&self, context: &Context) -> SearchQuery {
        println!("LLMOperations::impl::GoogleStudioLLM");
        let _ = self.generate_search_queries(context.to_owned()).await;
        todo!();
    }

    // fn identify_relevant_results(
    //     &self,
    //     context: &Context,
    //     search_result: &SearchResult,
    // ) -> Vec<RelevantFile> {
    //     // Anthropic-specific implementation
    // }

    // fn decide_continue_search(&self, context: &Context) -> bool {
    //     // Anthropic-specific implementation
    // }
}

pub struct GoogleStudioPlanGenerator {
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
        format!(
            r#"You will generate a search plan based on the provided context and user_query.
You will response with a search plan and a list of files that you want to search, in the following format:
<reply>
<search_plan>
</search_plan>
<files>
<path>
</path>
<path>
</path>
<path>
</path>
</files>
</reply>
        "#
        )
    }

    fn user_message_for_keyword_search(&self, request: &SearchPlanQuery) -> String {
        let context = request
            .context()
            .iter()
            .map(|c| match c {
                SearchPlanContext::RepoTree(repo_tree) => format!(
                    r#"RepoTree:
{}"#,
                    repo_tree
                ),
            })
            .collect::<Vec<String>>()
            .join("\n");

        format!(
            r#"User query: {}
Context:
{}"#,
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
                    ("event_type".to_owned(), "generate_search_plan".to_owned()),
                    ("root_id".to_owned(), root_request_id),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await?;

        println!("Generate search plan response time: {:?}", start.elapsed());

        SearchPlanResponse::parse(&response)
    }
}
