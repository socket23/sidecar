use crate::chunking::text_document::Position;
use async_recursion::async_recursion;
use futures::{stream, StreamExt};
use thiserror::Error;

use super::helpers::{guess_content, ProbableFileKind};

#[derive(Debug, Error)]
pub enum UserContextError {
    #[error("Unable to read from path: {0}")]
    UnableToReadFromPath(String),
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum VariableType {
    File,
    CodeSymbol,
    Selection,
}

impl VariableType {
    pub fn selection(&self) -> bool {
        self == &VariableType::Selection
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct VariableInformation {
    pub start_position: Position,
    pub end_position: Position,
    pub fs_file_path: String,
    pub name: String,
    #[serde(rename = "type")]
    pub variable_type: VariableType,
    pub content: String,
    pub language: String,
}

impl VariableInformation {
    pub fn is_selection(&self) -> bool {
        self.variable_type == VariableType::Selection
    }

    pub fn to_xml(self) -> String {
        let mut xml = "".to_owned();
        let variable_name = self.name;
        let location = format!(
            "{}:{}-{}",
            self.fs_file_path,
            self.start_position.line(),
            self.end_position.line(),
        );
        let content = self.content;
        let language = self.language;

        match self.variable_type {
            VariableType::CodeSymbol => {
                format!(
                    r#"<selection_item>
<symbol>
<name>
{variable_name}
</name>
<type>
Code Symbol
</type>
<file_path>
{location}
</file_path>
<content>
```{language}
{content}
```
</content>
</selection_item>"#
                )
            }
            VariableType::File => {
                format!(
                    r#"<selection_item>
<file>
<name>
{variable_name}
</name>
<file_path>
{location}
</file_path>
<content>
```{language}
{content}
```
</content>
</file>
</selection_item>"#
                )
            }
            VariableType::Selection => {
                format!(
                    r#"<selection_item>
<user_selection>
<name>
{variable_name}
</name>
<file_path>
{location}
</file_path>
<content>
```{language}
{content}
```
</content>
</user_selection>
</selection_item>"#
                )
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FileContentValue {
    pub file_path: String,
    pub file_content: String,
    pub language: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct UserContext {
    pub variables: Vec<VariableInformation>,
    pub file_content_map: Vec<FileContentValue>,
    pub terminal_selection: Option<String>,
    // These paths will be absolute and need to be used to get the
    // context of the folders here, we will output it properly
    folder_paths: Vec<String>,
}

impl UserContext {
    pub fn folder_paths(&self) -> Vec<String> {
        self.folder_paths.to_vec()
    }

    pub fn is_empty(&self) -> bool {
        self.variables.is_empty() && self.terminal_selection.is_none()
    }

    // generats the full xml for the input context so the llm can query from it
    pub async fn to_xml(self) -> Result<String, UserContextError> {
        let variable_prompt = self
            .variables
            .into_iter()
            .map(|variable| variable.to_xml())
            .collect::<Vec<_>>()
            .join("\n");
        let folder_content = stream::iter(self.folder_paths)
            .map(|folder_path| read_folder_selection(folder_path))
            .buffered(1)
            .collect::<Vec<Result<String, UserContextError>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, UserContextError>>()?
            .join("\n");
        // Now we create the xml string for this
        let mut final_string = "<selection>\n".to_owned();
        final_string.push_str(&variable_prompt);
        final_string.push_str("\n");
        final_string.push_str(&folder_content);
        final_string.push_str("\n</selection>");
        Ok(final_string)
    }
}

#[async_recursion]
pub async fn read_folder_selection(folder_path: String) -> Result<String, UserContextError> {
    let mut output = String::new();
    output.push_str(&format!(
        "<selection_item>\n<folder>\n<name>\n{}\n</name>\n<file_content>\n",
        folder_path
    ));

    let mut entries = tokio::fs::read_dir(folder_path.to_owned())
        .await
        .map_err(|_| UserContextError::UnableToReadFromPath(folder_path.to_owned()))?;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|_| UserContextError::UnableToReadFromPath(folder_path.to_owned()))?
    {
        let path = entry.path();

        if path.is_file() {
            let file_path = path.to_str().unwrap().to_owned();
            if path
                .extension()
                .map(|extension| extension == "json")
                .unwrap_or_default()
            {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|_| UserContextError::UnableToReadFromPath(file_path.clone()));
                if let Ok(content) = content {
                    if content.lines().collect::<Vec<_>>().len() >= 50 {
                        // just grab the first 50 lines and push it to the contet
                        output.push_str(&format!(
                            "<file_content>\n<file_path>\n{}\n</file_path>\n<content>\n{}\nTruncated...\n</content>\n</file_content>\n",
                            file_path, content.lines().take(50).collect::<Vec<_>>().join("\n")
                        ));
                    } else {
                        output.push_str(&format!(
                            "<file_content>\n<file_path>\n{}\n</file_path>\n<content>\n{}</content>\n</file_content>\n",
                            file_path, content
                        ));
                    }
                }
                // if we are in the json flow, then we have already consumed
                // this file and should break
                continue;
            }
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|_| UserContextError::UnableToReadFromPath(file_path.clone()));
            if let Ok(content) = content {
                let content_type = guess_content(content.as_bytes());
                match content_type {
                    ProbableFileKind::Text(content) => {
                        output.push_str(&format!(
                            "<file_content>\n<file_path>\n{}\n</file_path>\n<content>\n{}</content>\n</file_content>\n",
                            file_path, content
                        ));
                    }
                    ProbableFileKind::Binary => {
                        output.push_str(&format!(
                            "<file_content>\n<file_path>\n{}\n</file_path>\n<content>\nBinary Blob</content>\n</file_content>\n",
                            file_path
                        ));
                    }
                }
            }
        } else if path.is_dir() {
            let sub_folder_output = read_folder_selection(
                path.to_str()
                    .expect("might not work on windows :(")
                    .to_owned(),
            )
            .await?;
            output.push_str(&sub_folder_output);
        }
    }

    output.push_str("</file_content>\n</folder>\n</selection_item>");
    Ok(output)
}
