use sidecar::repomap::{files::SimpleFileSystem, types::RepoMap};
use std::path::PathBuf;

fn main() {
    let fs = Box::new(SimpleFileSystem);

    let repomap = RepoMap::new(fs);

    // change this to the directory you want to generate a repomap for
    let dir = PathBuf::from("src/repomap");
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let _ = repomap.generate(&project_root.join(&dir));
}
