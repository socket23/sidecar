use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage, LLMType},
    provider::{GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys},
};
use serde_xml_rs::{from_str, to_string};
use std::{sync::Arc, time::Instant};

use crate::{
    agent::search,
    agentic::{
        symbol::identifier::LLMProperties,
        tool::search::{
            agentic::{SearchPlanContext, SerdeError},
            exp::File,
            identify::IdentifyResponse,
        },
    },
};

use super::{
    agentic::{GenerateSearchPlan, GenerateSearchPlanError, SearchPlanQuery, SearchPlanResponse},
    decide::DecideResponse,
    exp::{
        Context, IterativeSearchError, LLMOperations, SearchQuery, SearchRequests, SearchResult,
    },
    identify::IdentifiedFile,
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

4. Ensure At Least One Tool:
Make sure that at least one of File or Keyword is provided. File allows you to search for file names. Keyword allows you to search for symbols such as class and function names.
You may use a combination of both.

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
<tool>Keyword</tool>
<query>
generate_report
</query>
</request>
<request>
<thinking>
</thinking>
<tool>File</tool>
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
        let file_context_string = File::serialise_files(context.files(), "\n");
        format!(
            r#"<issue>
{}
</issue>
<file_context>
{}
</file_context
        "#,
            context.user_query(),
            file_context_string
        )
    }

    pub fn system_message_for_identify(&self, context: &Context) -> String {
        format!(
            r#"You are an autonomous AI assistant tasked with finding relevant code in an existing 
codebase based on a reported issue. Your task is to identify the relevant code spans in the provided search 
results and decide whether the search task is complete.

# Input Structure:

* <issue>: Contains the reported issue.
* <file_context>: Contains the context of already identified files and code spans.
* <search_results>: Contains the new search results with code divided into "...............".

# Your Task:

1. Analyze User Instructions:
Carefully read the reported issue within the <issue> tag.

2. Review Current Context:
Examine the current file context provided in the <file_context> tag to understand already identified relevant files.

3. Process New Search Results:
3.1. Thoroughly analyze each code span in the <search_results> tag.
3.2. Match the code spans with the key elements, functions, variables, or patterns identified in the reported issue.
3.3. Evaluate the relevance of each code span based on how well it aligns with the reported issue and current file context.
3.4. If the issue suggests new functions or classes, identify the existing code that might be relevant to be able to implement the new functionality.
3.5. Review entire sections of code, not just isolated spans, to ensure you have a complete understanding before making a decision. It's crucial to see all code in a section to accurately determine relevance and completeness.
3.6. Verify if there are references to other parts of the codebase that might be relevant but not found in the search results. 
3.7. Identify and extract relevant code spans based on the reported issue. 

4. Response format:
<reply>
<responses>
<response>
<path>
</path>
<thinking>
</thinking>
</response>
<response>
<path>
</path>
<thinking>
</thinking>
</response>
<response>
<path>
</path>
<thinking>
</thinking>
</response>
</responses>
</reply>

Think step by step and write out your thoughts in the scratch_pad field."#
        )
    }

    pub fn user_message_for_identify(
        &self,
        context: &Context,
        search_results: &[SearchResult],
    ) -> String {
        let serialized_results: Vec<String> = search_results
            .iter()
            .filter_map(|r| match to_string(r) {
                Ok(s) => Some(GoogleStudioLLM::strip_xml_declaration(&s).to_string()),
                Err(e) => {
                    eprintln!("Error serializing SearchResult: {:?}", e);
                    None
                }
            })
            .collect();

        format!(
            r#"<issue>
{}
</issue>
<file_context>
{}
</file_context>
<search_results>
{}
</search_results>
"#,
            context.user_query(),
            context.file_paths_as_strings().join(", "),
            serialized_results.join("\n")
        )
    }

    pub fn system_message_for_decide(&self, context: &Context) -> String {
        format!(
            r#"You will be provided a reported issue and the file context containing existing code from the project's git repository. 
Your task is to make a decision if the code related to a reported issue is provided in the file context. 

# Input Structure:

* <issue>: Contains the reported issue.
* <file_context>: The file context.

Instructions:
    * Analyze the Issue:
    * Review the reported issue to understand what functionality or bug fix is being requested.

    * Analyze File Context:
    * Examine the provided file context to identify if the relevant code for the reported issue is present.
    * If the issue suggests that code should be implemented and doesn't yet exist in the code, consider the task completed if relevant code is found that would be modified to implement the new functionality.
    * If relevant code in the file context points to other parts of the codebase not included, note these references.

    * Make a Decision:
    * Decide if the relevant code is found in the file context.
    * If you believe all existing relevant code is identified, mark the task as complete.
    * If the specific method or code required to fix the issue is not present, still mark the task as complete as long as the relevant class or area for modification is identified.
    * If you believe more relevant code can be identified, mark the task as not complete and provide your suggestions on how to find the relevant code.

Important:
    * You CANNOT change the codebase. DO NOT modify or suggest changes to any code.
    * Your task is ONLY to determine if the file context is complete. Do not go beyond this scope.
    
Response format: 
<reply>
<response>
<suggestions>
</suggestions>
<complete>
</complete>
</response>
</reply>

Example:

<reply>
<response>
<suggestions>
We need to look for the method in another file
</suggestions>
<complete>
false
</complete>
</response>
</reply>
    "#
        )
    }

    pub fn user_message_for_decide(&self, context: &Context) -> String {
        let files = context.files();
        let serialised_files = File::serialise_files(files, "\n");

        format!(
            r#"<user_query>
{}
</user_query>
<file_context>
{}
</file_context
        "#,
            context.user_query(),
            serialised_files
        )
    }

    pub async fn generate_search_queries(
        &self,
        context: &Context,
    ) -> Result<Vec<SearchQuery>, IterativeSearchError> {
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
            .await?;

        Ok(GoogleStudioLLM::parse_search_response(&response)?.requests)
    }

    fn parse_search_response(response: &str) -> Result<SearchRequests, IterativeSearchError> {
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

    fn parse_identify_response(response: &str) -> Result<IdentifyResponse, IterativeSearchError> {
        let lines = response
            .lines()
            .skip_while(|l| !l.contains("<reply>"))
            .skip(1)
            .take_while(|l| !l.contains("</reply>"))
            .collect::<Vec<&str>>()
            .join("\n");

        from_str::<IdentifyResponse>(&lines).map_err(|error| {
            eprintln!("{:?}", error);
            IterativeSearchError::SerdeError(SerdeError::new(error, lines))
        })
    }

    fn parse_decide_response(response: &str) -> Result<DecideResponse, IterativeSearchError> {
        let lines = response
            .lines()
            .skip_while(|l| !l.contains("<reply>"))
            .skip(1)
            .take_while(|l| !l.contains("</reply>"))
            .collect::<Vec<&str>>()
            .join("\n");

        from_str::<DecideResponse>(&lines).map_err(|error| {
            eprintln!("{:?}", error);
            IterativeSearchError::SerdeError(SerdeError::new(error, lines))
        })
    }

    pub async fn identify(
        &self,
        context: &Context,
        search_results: &[SearchResult],
    ) -> Result<Vec<IdentifiedFile>, IterativeSearchError> {
        println!("GoogleStudioLLM::identify");

        let system_message = LLMClientMessage::system(self.system_message_for_identify(&context));

        // may need serde serialise!
        let user_message =
            LLMClientMessage::user(self.user_message_for_identify(&context, search_results));

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
                    ("event_type".to_owned(), "identify".to_owned()),
                    ("root_id".to_owned(), self.root_request_id.to_string()),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await?;

        Ok(GoogleStudioLLM::parse_identify_response(&response)?.responses)
    }

    pub async fn decide(
        &self,
        context: &mut Context,
    ) -> Result<DecideResponse, IterativeSearchError> {
        println!("GoogleStudioLLM::decide");

        let system_message = LLMClientMessage::system(self.system_message_for_decide(&context));

        let user_message = LLMClientMessage::user(self.user_message_for_decide(&context));

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
                    ("event_type".to_owned(), "decide".to_owned()),
                    ("root_id".to_owned(), self.root_request_id.to_string()),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await?;

        Ok(GoogleStudioLLM::parse_decide_response(&response)?)
    }

    pub fn strip_xml_declaration(input: &str) -> &str {
        const XML_DECLARATION_START: &str = "<?xml";
        const XML_DECLARATION_END: &str = "?>";

        if input.starts_with(XML_DECLARATION_START) {
            if let Some(end_pos) = input.find(XML_DECLARATION_END) {
                let start_pos = end_pos + XML_DECLARATION_END.len();
                input[start_pos..].trim_start()
            } else {
                input
            }
        } else {
            input
        }
    }
}

#[async_trait]
impl LLMOperations for GoogleStudioLLM {
    async fn generate_search_query(
        &self,
        context: &Context,
    ) -> Result<Vec<SearchQuery>, IterativeSearchError> {
        self.generate_search_queries(context).await
    }

    async fn identify_relevant_results(
        &self,
        context: &Context,
        search_results: &[SearchResult],
    ) -> Result<Vec<IdentifiedFile>, IterativeSearchError> {
        self.identify(context, search_results).await
    }

    async fn decide_continue(
        &self,
        context: &mut Context,
    ) -> Result<DecideResponse, IterativeSearchError> {
        self.decide(context).await
    }
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
