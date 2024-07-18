use std::collections::HashSet;
use std::path::PathBuf;

use super::graph::TagGraph;
use super::tag::{Tag, TagIndex};

pub struct TagAnalyzer {
    tag_index: TagIndex,
    tag_graph: TagGraph,
}

impl TagAnalyzer {
    pub fn new(tag_index: TagIndex) -> Self {
        let tag_graph = TagGraph::from_tag_index(&tag_index, &HashSet::new());
        Self {
            tag_index,
            tag_graph,
        }
    }

    pub fn get_ranked_tags(&mut self) -> Vec<&HashSet<Tag>> {
        self.tag_graph.calculate_and_distribute_ranks();
        let sorted_definitions = self.tag_graph.get_sorted_definitions();
        let graph = self.tag_graph.get_graph();

        let mut tags = vec![];

        for ((node, tag_name), _rank) in sorted_definitions {
            let path = PathBuf::from(&graph[*node]);
            if let Some(definition) = self.tag_index.definitions.get(&(path, tag_name.clone())) {
                tags.push(definition);
            }
        }

        tags
    }

    // Add other methods that require both TagIndex and TagGraph...
}
