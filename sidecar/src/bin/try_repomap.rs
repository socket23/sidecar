use sidecar::repomap::{files::SimpleFileSystem, types::RepoMap};
use std::path::PathBuf;

fn main() {
    let fs = Box::new(SimpleFileSystem);

    let repomap = RepoMap::new(fs).with_map_tokens(1000);

    // change this to the directory you want to generate a repomap for
    let dir = PathBuf::from("src/repomap");
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let full_path = project_root.join(&dir);

    let _ = repomap.get_repo_map(&full_path);
}
