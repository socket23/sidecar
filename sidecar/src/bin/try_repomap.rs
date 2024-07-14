use sidecar::{chunking::languages::TSLanguageParsing, repomap::types::RepoMap};
use std::{path::PathBuf, sync::Arc};
fn main() {
    let repomap = RepoMap::new(PathBuf::new());

    let ts_parsing = Arc::new(TSLanguageParsing::init());

    let fname_path = PathBuf::from("src/repomap/types.rs");

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let full_path = project_root.join(&fname_path);

    println!("{:?}", full_path);

    repomap.get_ranked_tags(&[full_path.clone()], &[full_path], ts_parsing);
}
