use sidecar::{
    chunking::languages::TSLanguageParsing,
    repomap::{analyser::TagGraph, tag::TagIndex, types::RepoMap},
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

    repomap.get_ranked_tags(&file_paths, &file_paths, ts_parsing, &mut tag_index);

    let mut tag_graph = TagGraph::from_tag_index(&tag_index, &HashSet::new());

    let ranks = tag_graph.calculate_page_ranks();

    tag_graph.calculate_and_distribute_ranks();

    dbg!(tag_graph.get_sorted_definitions());

    // tag_graph.get_ranked_tags(&tag_index);

    // analyser
    //     .save_dot("graph_with_weights.dot")
    //     .expect("Failed to save graph");
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
