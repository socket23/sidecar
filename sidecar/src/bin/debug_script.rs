use futures::{AsyncBufReadExt, StreamExt};

#[tokio::main]
async fn main() {
    let file_path = "/Users/skcd/scratch/sidecar/sidecar/src/webserver/tree_sitter.rs";
    let content = tokio::fs::read(file_path).await.expect("to work");
    let num_lines = String::from_utf16_lossy(content)
        .to_string()
        .lines()
        .collect::<Vec<_>>()
        .len();
    println!("num_lines: {}", num_lines);
}
