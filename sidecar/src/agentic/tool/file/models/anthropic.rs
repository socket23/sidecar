use std::{sync::Arc, time::Instant};

use axum::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::file::{
        file_finder::{ImportantFilesFinder, ImportantFilesFinderQuery},
        important::FileImportantResponse,
        types::{FileImportantError, SerdeError},
    },
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename = "files")]
pub struct FileImportantReply {
    #[serde(default)]
    files: Vec<String>,
}

impl FileImportantReply {
    pub fn parse_response(response: &str) -> Result<Self, FileImportantError> {
        if response.is_empty() {
            return Err(FileImportantError::EmptyResponse);
        }

        let lines = response
            .lines()
            .skip_while(|line| !line.contains("<files>"))
            .skip(1)
            .take_while(|line| !line.contains("</files>"))
            .map(|line| line.to_owned())
            .collect::<Vec<String>>();

        Ok(Self { files: lines })
    }

    pub fn files(&self) -> &Vec<String> {
        &self.files
    }

    pub fn to_file_important_response(self) -> FileImportantResponse {
        let files = self.files().clone();
        FileImportantResponse::new(files)
    }
}

pub struct AnthropicFileFinder {
    llm_client: Arc<LLMBroker>,
    fail_over_llm: LLMProperties,
}

impl AnthropicFileFinder {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            fail_over_llm,
        }
    }

    fn system_message_for_file_important(
        &self,
        file_important_request: &ImportantFilesFinderQuery,
    ) -> String {
        format!(
            r#"Observe the repository tree, and list the files you'd want might want to explore in order to solve the user query. 
            Any file that may be relevant. 
            Use your existing knowledge and intuition of the {} repository
            
            Respond in the following XML format:

            <files>
            User/sidecar/file1/
            User/sidecar/file2/
            User/sidecar/file3/
            </files>


            Notice how each xml tag ends with a new line, follow this format strictly.
            
            Response:

            <files>
        "#,
            file_important_request.repo_name()
        )
    }

    fn user_message_for_file_important(
        &self,
        file_important_request: &ImportantFilesFinderQuery,
    ) -> String {
        format!("{}", file_important_request.tree())
    }
}

#[async_trait]
impl ImportantFilesFinder for AnthropicFileFinder {
    async fn find_important_files(
        &self,
        request: ImportantFilesFinderQuery,
    ) -> Result<FileImportantResponse, FileImportantError> {
        let root_request_id = request.root_request_id().to_owned();
        let model = request.llm().clone();
        let provider = request.provider().clone();
        let api_keys = request.api_keys().clone();
        let system_message =
            LLMClientMessage::system(self.system_message_for_file_important(&request));
        let user_message = LLMClientMessage::user(self.user_message_for_file_important(&request));
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
                    ("event_type".to_owned(), "repo_map_search".to_owned()),
                    ("root_id".to_owned(), root_request_id.clone()),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await?;

        println!("File important response time: {:?}", start.elapsed());

        let parsed_response = FileImportantReply::parse_response(&response);

        match parsed_response {
            Ok(parsed_response) => Ok(parsed_response.to_file_important_response()),
            Err(e) => Err(e),
        }
    }
}
