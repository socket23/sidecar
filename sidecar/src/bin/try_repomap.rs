use sidecar::repomap::{tag::TagIndex, types::RepoMap};
use std::{collections::HashMap, path::Path};

#[tokio::main]
async fn main() {
    // let full_path = Path::new("/Users/zi/codestory/sidecar/sidecar/src");
    // let tag_index = TagIndex::from_path(full_path).await;
    let files = vec![
        "/Users/zi/codestory/sidecar/sidecar/src/repomap/tag.rs".to_owned(),
        "/Users/zi/codestory/sidecar/sidecar/src/repomap/types.rs".to_owned(),
    ];

    let root_path = Path::new("/Users/zi/codestory/sidecar/sidecar/src");
    let tag_index = TagIndex::from_files(root_path, files).await;

    let repomap = RepoMap::new().with_map_tokens(5_000);

    let repomap_string = repomap.get_repo_map(&tag_index).await.unwrap();

    println!("{}", repomap_string);
}
