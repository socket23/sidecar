pub type NodeIndex = petgraph::graph::NodeIndex<u32>;

enum EdgeType {
    /// If the code symbol uses the other code symbol
    /// for example: def hacking(a: Coder) means that we
    /// have an edge going from `hacking` -> `Coder`
    /// since hacking uses a Coder
    Uses,
    /// If we have def hacking(a: Coder), then `Coder` is refered
    /// in `hacking`, and that's another edge (we want bi-directional
    /// edges at this point)
    Refered,
    /// These are the LSP powered edges
    /// Go-to-Definition edge
    GoToDefinition,
    /// Go-to-references edge
    GoToReferences,
}

pub struct CodeGraphEdge {
    r#type: EdgeType,
    node_destination: NodeIndex,
}

pub struct CodeSymbolMetadata {
    fs_path: String,
    range: Range,
    outline_node: OutlineNode,
}

pub struct CodeSymbol {
    symbol_id: NodeIndex,
    edges: Vec<CodeGraphEdge>,
    metadata: CodeSymbolMetadata,
}

use dashmap::DashMap;
use petgraph::graph::DiGraph;
use petgraph::visit::EdgeRef;

use crate::chunking::text_document::Range;
use crate::chunking::types::OutlineNode;

// TODO(skcd): We need to improve this, the crux here is that we need to have
// a way to lock the code symbol down and unlock it so we can being editing it
// in a way we want to stop any callers which are already owning the lock from making
// progress and terminate the action they are about to do
// and then also allow for taking back control
pub struct CodeGraph {
    graph: DiGraph<NodeIndex, CodeGraphEdge>,
    locks: DashMap<NodeIndex, CodeSymbol>,
}

impl CodeGraph {
    pub fn new() -> Self {
        CodeGraph {
            graph: DiGraph::new(),
            locks: DashMap::new(),
        }
    }

    pub fn add_node(&mut self, code_symbol: CodeSymbol) -> NodeIndex {
        self.graph.add_node(code_symbol.symbol_id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = NodeIndex> {
        self.graph.node_indices()
    }
}
