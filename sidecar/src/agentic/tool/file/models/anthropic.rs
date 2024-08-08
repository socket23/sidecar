use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use async_trait::async_trait;
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::file::{
        file_finder::{ImportantFilesFinder, ImportantFilesFinderQuery},
        important::FileImportantResponse,
        types::FileImportantError,
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

    fn pathbuf_vec_to_string_vec(paths: Vec<PathBuf>) -> Vec<String> {
        paths
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect()
    }

    pub fn prepend_root_dir(&self, root: &Path) -> Self {
        let new_files: Vec<PathBuf> = self
            .files
            .iter()
            .map(|file| {
                let file_path = Path::new(file);
                if file_path.is_absolute() {
                    root.join(file_path.strip_prefix("/").unwrap_or(file_path))
                } else {
                    root.join(file_path)
                }
            })
            .collect();

        let new_files = FileImportantReply::pathbuf_vec_to_string_vec(new_files);

        Self { files: new_files }
    }

    pub fn to_file_important_response(self) -> FileImportantResponse {
        let files = self.files().clone();
        FileImportantResponse::new(files)
    }
}

pub struct AnthropicFileFinder {
    llm_client: Arc<LLMBroker>,
    _fail_over_llm: LLMProperties,
}

impl AnthropicFileFinder {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            _fail_over_llm: fail_over_llm,
        }
    }

    fn system_message_for_file_important(
        &self,
        _file_important_request: &ImportantFilesFinderQuery,
    ) -> String {
        format!(
            r#"Provided is a list of files in a repository.

Your job is to list the files you'd want to explore in order to solve the user query. If you're unsure, make an educated guess.
            
You must return at least 1 file, but no more than 10, in order of relevance.

Do not hallucinate files that do not exist in the provided tree.
            
Respond in the following XML format:

<files>
path/to/file1
path/to/file2
path/to/file3
</files>


Notice how each xml tag ends with a new line, follow this format strictly.

Response:

<files>
"#,
        )
    }

    fn user_message_for_file_important(
        &self,
        file_important_request: &ImportantFilesFinderQuery,
    ) -> String {
        format!(
            "User query: {}\n\nTree:\n{}",
            file_important_request.user_query(),
            file_important_request.tree()
        )
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
                    ("event_type".to_owned(), "important_file_finder".to_owned()),
                    ("root_id".to_owned(), root_request_id),
                ]
                .into_iter()
                .collect(),
                sender,
            )
            .await?;

        println!("file_important_broker::time_take({:?})", start.elapsed());

        println!("{}", response);

        let parsed_response = FileImportantReply::parse_response(&response);

        if let Ok(response) = &parsed_response {
            for (index, f) in response.files().iter().enumerate() {
                println!("File {}: {}", index, f);
            }
        } else {
            println!("could not parse response");
        }

        match parsed_response {
            Ok(parsed_response) => Ok(parsed_response
                .prepend_root_dir(Path::new(request.repo_name()))
                .to_file_important_response()),
            Err(e) => Err(e),
        }
    }
}
