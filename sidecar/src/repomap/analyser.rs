use petgraph::algo::page_rank::page_rank;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::tag::TagIndex;

pub struct TagGraph {
    graph: DiGraph<String, f64>,
    node_indices: HashMap<String, NodeIndex>,
}

impl TagGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn from_tag_index(tag_index: &TagIndex, mentioned_idents: &HashSet<String>) -> Self {
        let mut tag_graph = Self::new();
        tag_graph.populate_from_tag_index(tag_index, mentioned_idents);
        tag_graph
    }

    // TODO - petgraph does not support personalization vectors

    pub fn populate_from_tag_index(
        &mut self,
        tag_index: &TagIndex,
        mentioned_idents: &HashSet<String>,
    ) {
        let mut edge_count = 0;
        for ident in &tag_index.common_tags {
            let mul = self.calculate_multiplier(ident, mentioned_idents);
            let num_refs = tag_index.references[ident].len() as f64;
            let scaled_refs = num_refs.sqrt();

            println!(
                "Ident: {} has {} references and {} defines",
                ident,
                num_refs,
                tag_index.defines[ident].len()
            );

            for referencer in &tag_index.references[ident] {
                for definer in &tag_index.defines[ident] {
                    let referencer_idx = self.get_or_create_node(referencer.to_str().unwrap());
                    let definer_idx = self.get_or_create_node(definer.to_str().unwrap());

                    println!(
                        "Adding edge {} from {} to {}. Weight: {} \n\n",
                        edge_count + 1,
                        referencer.display(),
                        definer.display(),
                        mul * scaled_refs
                    );

                    self.graph
                        .add_edge(referencer_idx, definer_idx, mul * scaled_refs);
                    edge_count += 1;
                }
            }
        }
    }

    pub fn calculate_page_ranks(&self) -> Vec<f64> {
        page_rank(&self.graph, 0.85, 100)
    }

    pub fn distribute_rank(&self, ranked: &Vec<f64>) -> Vec<((NodeIndex, String), f64)> {
        let mut ranked_definitions: HashMap<(NodeIndex, String), f64> = HashMap::new();

        for src in self.graph.node_indices() {
            let src_rank = ranked[src.index() as usize];

            println!("Source: {:?} has rank: {}", src, src_rank);
            let total_weight: f64 = self.graph.edges(src).map(|edge| *edge.weight()).sum();

            for edge in self.graph.edges(src) {
                println!("Edge: {:?}", edge);
                let dst = edge.target();
                let weight = *edge.weight();
                // let rank = src_rank * weight / total_weight;

                // Assuming you have a way to get the 'ident' for each edge
                // let ident = get_ident_for_edge(edge);

                // *ranked_definitions.entry((dst, ident)).or_insert(0.0) += rank;
            }
        }

        let mut ranked_tags: Vec<((NodeIndex, String), f64)> =
            ranked_definitions.into_iter().collect();
        ranked_tags.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        ranked_tags
    }

    pub fn generate_dot_representation(&self) -> String {
        let mut dot = String::new();
        writeln!(&mut dot, "digraph {{").unwrap();

        for node_index in self.graph.node_indices() {
            let node_label = &self.graph[node_index];
            writeln!(
                &mut dot,
                "    {:?} [ label = {:?} ]",
                node_index.index(),
                node_label
            )
            .unwrap();
        }

        for edge in self.graph.edge_references() {
            let (source, target) = (edge.source().index(), edge.target().index());
            let weight = edge.weight();
            writeln!(
                &mut dot,
                "    {:?} -> {:?} [ label = {:?} ]",
                source, target, weight
            )
            .unwrap();
        }

        writeln!(&mut dot, "}}").unwrap();
        dot
    }

    pub fn print_dot(&self) {
        println!("{}", self.generate_dot_representation());
    }

    fn get_or_create_node(&mut self, name: &str) -> NodeIndex {
        *self
            .node_indices
            .entry(name.to_string())
            .or_insert_with(|| self.graph.add_node(name.to_string()))
    }

    fn calculate_multiplier(&self, tag: &str, mentioned_idents: &HashSet<String>) -> f64 {
        if mentioned_idents.contains(tag) {
            10.0
        } else if tag.starts_with('_') {
            0.1
        } else {
            1.0
        }
    }
}
