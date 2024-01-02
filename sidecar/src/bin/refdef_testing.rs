//! We are going to send a post request using reqwest here to the token information
//! endpoint and use that for checking out if our goto-ref and other things are
//! working as expected in a single file, if they work then we can scale it up
//! to multiple files

use sidecar::{
    chunking::text_document::Position,
    repo::types::RepoRef,
    webserver::{
        in_line_agent::{SnippetInformation, TextDocumentWeb},
        token_information::TokenInformationRequest,
    },
};

#[tokio::main]
async fn main() {
    let file_path = "/Users/skcd/scratch/sidecar/sidecar/src/bin/test_files/bloop_answer_b.rs";
    let relative_path = "sidecar/src/bin/test_files/bloop_answer_b.rs";
    let client = reqwest::Client::new();
    let text = std::fs::read_to_string(file_path).unwrap();
    let line_count = text.lines().count();
    let token_information = TokenInformationRequest {
        repo_ref: RepoRef::local("/dev/null").expect("to work"),
        relative_path: "src/webserver/in_line_agent.rs".to_string(),
        hovered_text: "Action".to_owned(),
        snippet_information: SnippetInformation {
            start_position: Position::new(140, 17, 0),
            end_position: Position::new(140, 23, 0),
        },
        text_document_web: TextDocumentWeb {
            text: text.to_owned(),
            utf8_array: text.as_bytes().to_vec(),
            language: "rust".to_owned(),
            fs_file_path: file_path.to_owned(),
            relative_path: relative_path.to_owned(),
            line_count,
        },
    };
    let res = dbg!(client
        .post("http://127.0.0.1:42424/api/navigation/token_information")
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&token_information).expect("to work"))
        .send()
        .await
        .expect("to work"));
}
