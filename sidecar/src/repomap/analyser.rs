use petgraph::algo::page_rank::page_rank;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::prelude::EdgeIndex;
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
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

    pub fn get_ranked_tags(&self) -> Vec<&HashSet<Tag>> {
        let ranked_definitions = self.tag_graph.get_ranked_definitions();
        let graph = self.tag_graph.get_graph();

        let mut tags = vec![];

        for ((node, tag_name), _rank) in ranked_definitions {
            let path = PathBuf::from(&graph[*node]);
            if let Some(definition) = self.tag_index.definitions.get(&(path, tag_name.clone())) {
                tags.push(definition);
            }
        }

        tags
    }

    // Add other methods that require both TagIndex and TagGraph...
}
