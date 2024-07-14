use sidecar::{chunking::languages::TSLanguageParsing, repomap::types::RepoMap};
use std::{path::PathBuf, sync::Arc};
fn main() {
    let repomap = RepoMap::new(PathBuf::new());

    let ts_parsing = Arc::new(TSLanguageParsing::init());

    let fname_path = PathBuf::from("src/repomap/types.rs");

    repomap.get_ranked_tags(&fname_path, ts_parsing);
}
