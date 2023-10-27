use regex::Regex;

use super::{
    languages::TSLanguageConfig,
    text_document::{DocumentSymbol, Range, TextDocument},
};

/// Here we will parse the document we get from the editor using symbol level
/// information, as its very fast

#[derive(Debug, Clone)]
struct EditorParsing {
    configs: Vec<TSLanguageConfig>,
}

impl EditorParsing {
    fn ts_language_config(&self, language: &str) -> Option<&TSLanguageConfig> {
        self.configs
            .iter()
            .find(|config| config.language_ids.contains(&language))
    }

    fn is_node_identifier(
        &self,
        node: &tree_sitter::Node,
        language_config: &TSLanguageConfig,
    ) -> bool {
        match language_config
            .language_ids
            .first()
            .expect("language_id to be present")
            .to_lowercase()
            .as_ref()
        {
            "typescript" | "typescriptreact" | "javascript" | "javascriptreact" => {
                Regex::new(r"(definition|declaration|declarator|export_statement)")
                    .unwrap()
                    .is_match(node.kind())
            }
            "golang" => Regex::new(r"(definition|declaration|declarator|var_spec)")
                .unwrap()
                .is_match(node.kind()),
            "cpp" => Regex::new(r"(definition|declaration|declarator|class_specifier)")
                .unwrap()
                .is_match(node.kind()),
            "ruby" => Regex::new(r"(module|class|method|assignment)")
                .unwrap()
                .is_match(node.kind()),
            _ => Regex::new(r"(definition|declaration|declarator)")
                .unwrap()
                .is_match(node.kind()),
        }
    }

    /**
     * This function aims to process nodes from a tree sitter parsed structure
     * based on their intersection with a given range and identify nodes that
     * represent declarations or definitions specific to a programming language.
     *
     * @param {Object} t - The tree sitter node.
     * @param {Object} e - The range (or point structure) with which intersections are checked.
     * @param {string} r - The programming language (e.g., "typescript", "golang").
     *
     * @return {Object|undefined} - Returns the most relevant node or undefined.
     */
    // function KX(t, e, r) {
    // // Initial setup with the root node and an empty list for potential matches
    // let n = [t.rootNode], i = [];

    // while (true) {
    //     // For each node in 'n', calculate its intersection size with 'e'
    //     let o = n.map(s => [s, rs.intersectionSize(s, e)])
    //              .filter(([s, a]) => a > 0)
    //              .sort(([s, a], [l, c]) => c - a);  // sort in decreasing order of intersection size

    //     // If there are no intersections, either return undefined or the most relevant node from 'i'
    //     if (o.length === 0) return i.length === 0 ? void 0 : tX(i, ([s, a], [l, c]) => a - c)[0];

    //     // For the nodes in 'o', calculate a relevance score and filter the ones that are declarations or definitions for language 'r'
    //     let s = o.map(([a, l]) => {
    //         let c = rs.len(a),  // Length of the node
    //             u = Math.abs(rs.len(e) - l),  // Difference between length of 'e' and its intersection size
    //             p = (l - u) / c;  // Relevance score
    //         return [a, p];
    //     });

    //     // Filter nodes based on the ZL function and push to 'i'
    //     i.push(...s.filter(([a, l]) => ZL(a, r)));

    //     // Prepare for the next iteration by setting 'n' to the children of the nodes in 'o'
    //     n = [];
    //     n.push(...s.flatMap(([a, l]) => a.children));
    // }
    // }
    fn get_identifier_node_fully_contained(
        &self,
        tree_sitter_node: tree_sitter::Node,
        range: &Range,
        language_config: &TSLanguageConfig,
    ) -> Option<tree_sitter::Node> {
        let mut nodes = vec![tree_sitter_node];
        let mut identifier_nodes: Vec<(tree_sitter::Node, f64)> = vec![];
        loop {
            // Here we take the nodes in [nodes] which have an intersection
            // with the range we are interested in
            let mut intersecting_nodes = nodes
                .into_iter()
                .map(|tree_sitter_node| {
                    (
                        tree_sitter_node,
                        Range::for_tree_node(&tree_sitter_node).intersection_size(range) as f64,
                    )
                })
                .filter(|(_, intersection_size)| intersection_size > &0.0)
                .collect::<Vec<_>>();
            // we sort the nodes by their intersection size
            // we want to keep the biggest size here on the top
            intersecting_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("partial_cmp to work"));

            // if there are no nodes, then we return none or the most relevant nodes
            // from i, which is the biggest node here
            if intersecting_nodes.is_empty() {
                return if identifier_nodes.is_empty() {
                    None
                } else {
                    Some({
                        identifier_nodes
                            .sort_by(|a, b| b.1.partial_cmp(&a.1).expect("partial_cmp to work"));
                        identifier_nodes.remove(0).0
                    })
                };
            }

            // For the nodes in o, calculate a relevance score and filter the ones that are declarations or definitions for language 'r'
            let mut identifier_nodes_sorted = intersecting_nodes
                .iter()
                .map(|(tree_sitter_node, intersection_size)| {
                    let len = Range::for_tree_node(&tree_sitter_node).len();
                    let diff = (range.len() as f64 - intersection_size) as f64;
                    let relevance_score = (intersection_size - diff) as f64 / len as f64;
                    (tree_sitter_node.clone(), relevance_score)
                })
                .collect::<Vec<_>>();

            // now we filter out the nodes which are here based on the identifier function and set it to i
            intersecting_nodes.extend(
                identifier_nodes_sorted
                    .drain(..)
                    .filter(|(tree_sitter_node, _)| {
                        self.is_node_identifier(tree_sitter_node, language_config)
                    })
                    .collect::<Vec<_>>(),
            );

            // Now we prepare for the next iteration by setting nodes to the children of the nodes
            // in intersecting_nodes
            nodes = intersecting_nodes
                .into_iter()
                .map(|(tree_sitter_node, _)| {
                    let mut cursor = tree_sitter_node.walk();
                    tree_sitter_node.children(&mut cursor).collect::<Vec<_>>()
                })
                .flatten()
                .collect::<Vec<_>>();
        }
    }

    fn get_identifier_node_by_expanding<'a>(
        &'a self,
        tree_sitter_node: tree_sitter::Node<'a>,
        range: &Range,
        language_config: &TSLanguageConfig,
    ) -> Option<tree_sitter::Node<'a>> {
        let tree_sitter_range = range.to_tree_sitter_range();
        let mut expanding_node = tree_sitter_node
            .descendant_for_byte_range(tree_sitter_range.start_byte, tree_sitter_range.end_byte);
        loop {
            // Here we expand this node until we hit a identifier node, this is
            // a very easy way to get to the best node we are interested in by
            // bubbling up
            if expanding_node.is_none() {
                return None;
            }
            match expanding_node {
                Some(expanding_node_val) => {
                    // if this is not a identifier and the parent is there, we keep
                    // going up
                    if !self.is_node_identifier(&expanding_node_val, &language_config)
                        && expanding_node_val.parent().is_some()
                    {
                        expanding_node = expanding_node_val.parent()
                    // if we have a node identifier, return right here!
                    } else if self.is_node_identifier(&expanding_node_val, &language_config) {
                        return Some(expanding_node_val.clone());
                    } else {
                        // so we don't have a node identifier and neither a parent, so
                        // just return None
                        return None;
                    }
                }
                None => {
                    return None;
                }
            }
        }
    }

    pub fn get_documentation_node(
        &self,
        text_document: &TextDocument,
        language_config: &TSLanguageConfig,
        range: Range,
    ) -> Vec<DocumentSymbol> {
        let language = language_config.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(language()).unwrap();
        let tree = parser
            .parse(text_document.get_content_buffer().as_bytes(), None)
            .unwrap();
        if let Some(identifier_node) =
            self.get_identifier_node_fully_contained(tree.root_node(), &range, &language_config)
        {
            // we have a identifier node right here, so lets get the document symbol
            // for this and return it back
            return DocumentSymbol::from_tree_node(
                &identifier_node,
                language_config,
                text_document.get_content_buffer(),
            )
            .into_iter()
            .collect();
        }
        // or else we try to expand the node out so we can get a symbol back
        if let Some(expanded_node) =
            self.get_identifier_node_by_expanding(tree.root_node(), &range, &language_config)
        {
            // we get the expanded node here again
            return DocumentSymbol::from_tree_node(
                &expanded_node,
                language_config,
                text_document.get_content_buffer(),
            )
            .into_iter()
            .collect();
        }
        // or else we return nothing here
        vec![]
    }
}
