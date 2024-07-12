use sidecar::repomap::types::RepoMap;
use std::path::PathBuf;

fn main() {
    let mut repomap = RepoMap::new(PathBuf::new());

    let res = repomap.parse_tree("python", "src/repomap/types.rs");
}
