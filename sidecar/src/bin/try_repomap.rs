use sidecar::{
    chunking::languages::TSLanguageParsing,
    repomap::{analyser::TagAnalyzer, tag::TagIndex, types::RepoMap},
};
use std::{collections::HashSet, fs::read_dir, path::PathBuf, sync::Arc};
fn main() {
    let repomap = RepoMap::new(PathBuf::new());

    let ts_parsing = Arc::new(TSLanguageParsing::init());

    let dir_path = FullPath::new(PathBuf::from("src/repomap"));

    let paths = read_dir(dir_path.path).unwrap();

    let mut file_paths: Vec<PathBuf> = paths
        .filter_map(|path| {
            let entry = path.unwrap();
            let path = entry.path();
            if path.is_dir() {
                return None;
            }
            Some(FullPath::new(path).path)
        })
        .collect();

    let extra_path = FullPath::new(PathBuf::from("src/bin/try_repomap.rs"));

    file_paths.push(extra_path.path);

    let mut tag_index = TagIndex::new();
}

struct FullPath {
    path: PathBuf,
}

impl FullPath {
    pub fn new(file_path_buf: PathBuf) -> FullPath {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        FullPath {
            path: project_root.join(&file_path_buf),
        }
    }
}
