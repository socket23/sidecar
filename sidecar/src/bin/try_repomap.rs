use sidecar::repomap::types::RepoMap;
use std::path::PathBuf;

fn main() {
    let mut repomap = RepoMap::new(PathBuf::new());

    let query = repomap.get_query("python");
    println!("Query: {}", query);
}
