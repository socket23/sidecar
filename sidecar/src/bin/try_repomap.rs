use sidecar::repomap::{tag::TagIndex, types::RepoMap};
use std::path::Path;

#[tokio::main]
async fn main() {
    let full_path = Path::new("/Users/zi/codestory/sidecar/sidecar/src");
    let tag_index = TagIndex::from_path(full_path).await;
    let repomap = RepoMap::new().with_map_tokens(30_000);

    let repomap_string = repomap.get_repo_map(&tag_index).await.unwrap();

    println!("{}", repomap_string);
}
