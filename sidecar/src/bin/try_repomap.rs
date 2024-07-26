use sidecar::repomap::types::RepoMap;
use std::path::Path;

#[tokio::main]
async fn main() {
    let repomap = RepoMap::new().with_map_tokens(1000);

    // change this to the directory you want to generate a repomap for
    // let dir = PathBuf::from(".");
    // let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // let full_path = project_root.join(&dir);

    let full_path = Path::new("/Users/skcd/scratch/sidecar");

    let repomap = repomap.get_repo_map(&full_path).await.unwrap();

    println!("{}", repomap);
}
