use sidecar::{chunking::languages::TSLanguageParsing, repomap::types::RepoMap};
use std::{path::PathBuf, sync::Arc};
fn main() {
    let repomap = RepoMap::new(PathBuf::new());

    let ts_parsing = Arc::new(TSLanguageParsing::init());

    let file_names = vec!["src/repomap/types.rs"];

    let paths: Vec<PathBuf> = file_names
        .iter()
        .map(|fname| FullPath::new(fname).path)
        .collect();

    repomap.get_ranked_tags(&paths, &paths, ts_parsing);
}

struct FullPath {
    path: PathBuf,
}

impl FullPath {
    pub fn new(fname: &str) -> FullPath {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path_buf = PathBuf::from(fname);
        FullPath {
            path: project_root.join(&path_buf),
        }
    }
}
