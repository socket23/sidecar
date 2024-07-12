use std::path::PathBuf;
use sidecar::repomap::types::RepoMap;

fn main() {
    let mut repomap = RepoMap::new(PathBuf::new());
    
    let query = repomap.get_query("python").unwrap();
    println!("Query: {}", query);
}