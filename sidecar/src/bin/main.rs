use sidecar::{embedder::embedder::Embedder, embedder::embedder::LocalEmbedder};
use std::env;

#[tokio::main]
async fn main() {
    println!("Hello, world! skcd");
    // what about now??
    // Now we try to create the embedder and see if thats working
    let current_path = env::current_dir().unwrap();
    // Checking that the embedding logic is also working
    let embedder = LocalEmbedder::new(&current_path.join("models/all-MiniLM-L6-v2/")).unwrap();
    let result = embedder.embed("hello world!").unwrap();
    let something = Some("something".to_owned());
}
