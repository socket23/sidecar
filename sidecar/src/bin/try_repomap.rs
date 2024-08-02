use sidecar::repomap::types::RepoMap;
use std::path::Path;

#[tokio::main]
async fn main() {
    let full_path = Path::new("/Users/skcd/scratch/sidecar");
    let mut repomap = RepoMap::new(full_path.to_str().unwrap().to_owned()).with_map_tokens(50_000);

    // change this to the directory you want to generate a repomap for
    // let dir = PathBuf::from(".");
    // let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // let full_path = project_root.join(&dir);

    let repomap = repomap.get_repo_map().await.unwrap();

    println!("{}", repomap);
}
