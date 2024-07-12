use sidecar::{chunking::languages::TSLanguageParsing, repomap::types::RepoMap};
use std::{path::PathBuf, sync::Arc};
fn main() {
    // let mut repomap = RepoMap::new(PathBuf::new());

    let ts_parsing = Arc::new(TSLanguageParsing::init());

    RepoMap::try_parsing("src/repomap/types.rs", ts_parsing);
}
