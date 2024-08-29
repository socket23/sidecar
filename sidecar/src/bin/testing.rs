use tokio::task;
use walkdir::WalkDir;
use std::path::Path;
use futures::stream::{self, StreamExt};
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;

#[tokio::main]
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use walkdir::WalkDir;
use std::path::PathBuf;

async fn main() {
	let root = "."; // Start from the current directory
	let file_stream = create_file_stream(root);
	process_files_in_parallel(file_stream, 10).await;
}

fn create_file_stream(root: &str) -> ReceiverStream<PathBuf> {
	let (tx, rx) = mpsc::channel(100); // Create a channel with a buffer of 100

	tokio::spawn(async move {
		for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
			if entry.file_type().is_file() {
				let path = entry.path().to_owned();
				if tx.send(path).await.is_err() {
					break; // If the receiver is closed, stop sending
				}
			}
		}
	});

	ReceiverStream::new(rx)
}

async fn process_files_in_parallel(file_stream: ReceiverStream<PathBuf>, concurrency: usize) {
	file_stream
		.map(|path| {
			tokio::spawn(async move {
				process_file(&path).await;
			})
		})
		.buffer_unordered(concurrency)
		.for_each(|_| async { /* Ignore the result */ })
		.await;
}

async fn process_file(path: &PathBuf) {
	// Implement file processing logic here
	println!("Processing file: {:?}", path);
}