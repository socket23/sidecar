use sidecar::{embedder::embedder::Embedder, embedder::embedder::LocalEmbedder};
use std::env;

#[tokio::main]
async fn main() {
    println!("Hello, world! skcd");
    init_ort_dylib();

    // Now we try to create the embedder and see if thats working
    let current_path = env::current_dir().unwrap();
    // Checking that the embedding logic is also working
    let embedder = LocalEmbedder::new(&current_path.join("models/all-MiniLM-L6-v2/")).unwrap();
    let result = embedder.embed("hello world!").unwrap();
    let something = Some("something".to_owned());
    dbg!(result);
}

fn init_ort_dylib() {
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "linux")]
        let lib_path = "libonnxruntime.so";
        #[cfg(target_os = "macos")]
        let lib_path =
            "/Users/skcd/Downloads/onnxruntime-osx-arm64-1.16.0/lib/libonnxruntime.dylib";

        // let ort_dylib_path = dylib_dir.as_ref().join(lib_name);

        if env::var("ORT_DYLIB_PATH").is_err() {
            env::set_var("ORT_DYLIB_PATH", lib_path);
        }
    }
}
