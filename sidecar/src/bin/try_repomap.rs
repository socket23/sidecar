use sidecar::{chunking::languages::TSLanguageParsing, repomap::types::RepoMap};
use std::{path::PathBuf, sync::Arc};
fn main() {
    let repomap = RepoMap::new(PathBuf::new());

    let ts_parsing = Arc::new(TSLanguageParsing::init());

    repomap.try_parsing("src/bin/try_repomap.rs", ts_parsing);
}
