use anchor_lang::prelude::*;

pub type NodeId = u128;

#[derive(Debug, Clone)]
pub struct TraverseFilter {
    pub where_node_labels: Vec<String>,
    pub where_edge_labels: Vec<String>,
    pub where_not_node_labels: Vec<String>,
    pub where_not_edge_labels: Vec<String>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub data: Vec<u8>,
    pub outgoing_edge_indices: Vec<u32>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub label: String,
}

#[account]
pub struct GraphStore {
    pub authority: Pubkey,
    pub node_count: u64,
    pub edge_count: u64,
    pub nonce: NodeId,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl GraphStore {
    pub fn get_node_by_id(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn traverse_out(
        &self,
        start_nodes: &[NodeId],
        filter: &TraverseFilter,
        limit: Option<usize>,
    ) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Check and add start nodes if they match the node label filters
        // (edge filters don't apply to start nodes since we don't traverse to them)
        for &node_id in start_nodes {
            if let Some(node) = self.get_node_by_id(node_id) {
                // Check node label filters for start nodes
                let node_matches = if !filter.where_node_labels.is_empty() {
                    filter.where_node_labels.contains(&node.label)
                } else {
                    true
                };

                let node_not_matches = if !filter.where_not_node_labels.is_empty() {
                    filter.where_not_node_labels.contains(&node.label)
                } else {
                    false
                };

                if node_matches && !node_not_matches {
                    result.push(node_id);
                }

                queue.push_back(node_id);
                visited.insert(node_id);
            }
        }

        // If edge filters are empty, we only filter start nodes, don't traverse
        let should_traverse =
            !filter.where_edge_labels.is_empty() || !filter.where_not_edge_labels.is_empty();

        if should_traverse {
            while let Some(current_id) = queue.pop_front() {
                if let Some(limit) = limit {
                    if result.len() >= limit {
                        break;
                    }
                }

                if let Some(current_node) = self.get_node_by_id(current_id) {
                    for &edge_index in &current_node.outgoing_edge_indices {
                        if let Some(edge) = self.edges.get(edge_index as usize) {
                            // Check edge label filters
                            let edge_matches = if !filter.where_edge_labels.is_empty() {
                                filter.where_edge_labels.contains(&edge.label)
                            } else {
                                true
                            };

                            let edge_not_matches = if !filter.where_not_edge_labels.is_empty() {
                                filter.where_not_edge_labels.contains(&edge.label)
                            } else {
                                false
                            };

                            if edge_matches && !edge_not_matches {
                                let target_id = edge.to;

                                if !visited.contains(&target_id) {
                                    visited.insert(target_id);

                                    if let Some(target_node) = self.get_node_by_id(target_id) {
                                        // Check node label filters
                                        let node_matches = if !filter.where_node_labels.is_empty() {
                                            filter.where_node_labels.contains(&target_node.label)
                                        } else {
                                            true
                                        };

                                        let node_not_matches =
                                            if !filter.where_not_node_labels.is_empty() {
                                                filter
                                                    .where_not_node_labels
                                                    .contains(&target_node.label)
                                            } else {
                                                false
                                            };

                                        if node_matches && !node_not_matches {
                                            result.push(target_id);

                                            if let Some(limit) = limit {
                                                if result.len() >= limit {
                                                    return result;
                                                }
                                            }

                                            queue.push_back(target_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn create_filter(node_label: &str, edge_label: &str) -> TraverseFilter {
        TraverseFilter {
            where_node_labels: vec![node_label.to_string()],
            where_edge_labels: vec![edge_label.to_string()],
            where_not_node_labels: Vec::new(),
            where_not_edge_labels: Vec::new(),
        }
    }

    // Test graph schema:
    //
    //     City(1) ──Railway──> City(2) ──Railway──> City(3)
    //       │                      │                    │
    //       │                      │                    │
    //       │                      └──Highway──> Town(4) │
    //       │                                           │
    //       └────────────Railway────────────────────────┘
    //                    (cycle)
    //
    //     Town(5) (isolated node)
    //
    fn create_small_test_graph() -> GraphStore {
        let authority = Pubkey::new_unique();

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        nodes.push(Node {
            id: 1,
            label: "City".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![0, 1],
        });

        nodes.push(Node {
            id: 2,
            label: "City".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![2, 3],
        });

        nodes.push(Node {
            id: 3,
            label: "City".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![4],
        });

        nodes.push(Node {
            id: 4,
            label: "Town".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![],
        });

        nodes.push(Node {
            id: 5,
            label: "Town".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![],
        });

        edges.push(Edge {
            from: 1,
            to: 2,
            label: "Railway".to_string(),
        });

        edges.push(Edge {
            from: 1,
            to: 3,
            label: "Railway".to_string(),
        });

        edges.push(Edge {
            from: 2,
            to: 3,
            label: "Railway".to_string(),
        });

        edges.push(Edge {
            from: 2,
            to: 4,
            label: "Highway".to_string(),
        });

        edges.push(Edge {
            from: 3,
            to: 1,
            label: "Railway".to_string(),
        });

        GraphStore {
            authority,
            node_count: 5,
            edge_count: 5,
            nonce: 6,
            nodes,
            edges,
        }
    }

    #[test]
    fn test_traverse_out_simple() {
        let graph = create_small_test_graph();

        let filter = create_filter("City", "Railway");
        let result = graph.traverse_out(&[1], &filter, None);

        assert_eq!(result.len(), 3);
        assert!(result.contains(&1)); // Start node is included
        assert!(result.contains(&2));
        assert!(result.contains(&3));
    }

    #[test]
    fn test_traverse_out_with_limit() {
        let graph = create_small_test_graph();

        let filter = create_filter("City", "Railway");
        let result = graph.traverse_out(&[1], &filter, Some(1));

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_traverse_out_wrong_edge_label() {
        let graph = create_small_test_graph();

        let filter = create_filter("City", "NONEXISTENT");
        let result = graph.traverse_out(&[1], &filter, None);

        assert_eq!(result.len(), 1);
        assert!(result.contains(&1)); // Start node is included even if no edges match
    }

    #[test]
    fn test_traverse_out_wrong_node_label() {
        let graph = create_small_test_graph();

        let filter = create_filter("Town", "Railway");
        let result = graph.traverse_out(&[1], &filter, None);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_traverse_out_multiple_start_nodes() {
        let graph = create_small_test_graph();

        let filter = create_filter("City", "Railway");
        let result = graph.traverse_out(&[1, 2], &filter, None);

        assert_eq!(result.len(), 3);
        assert!(result.contains(&1)); // Start node 1 is included
        assert!(result.contains(&2)); // Start node 2 is included
        assert!(result.contains(&3));
    }

    #[test]
    fn test_traverse_out_handles_cycles() {
        let graph = create_small_test_graph();

        let filter = create_filter("City", "Railway");
        let result = graph.traverse_out(&[1], &filter, None);

        assert_eq!(result.len(), 3);
        assert!(result.contains(&1)); // Start node is included
        assert!(result.contains(&2));
        assert!(result.contains(&3));
    }

    #[test]
    fn test_traverse_out_different_edge_types() {
        let graph = create_small_test_graph();

        let filter = create_filter("Town", "Highway");
        let result = graph.traverse_out(&[2], &filter, None);

        assert_eq!(result.len(), 1);
        assert!(result.contains(&4));
    }

    #[test]
    fn test_traverse_out_nonexistent_start_node() {
        let graph = create_small_test_graph();

        let filter = create_filter("City", "Railway");
        let result = graph.traverse_out(&[999], &filter, None);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_traverse_out_empty_start_nodes() {
        let graph = create_small_test_graph();

        let filter = create_filter("City", "Railway");
        let result = graph.traverse_out(&[], &filter, None);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_traverse_out_multi_hop() {
        let graph = create_small_test_graph();

        let filter = create_filter("City", "Railway");
        let result = graph.traverse_out(&[1], &filter, None);

        assert_eq!(result.len(), 3);
        assert!(result.contains(&1)); // Start node is included
        assert!(result.contains(&2));
        assert!(result.contains(&3));
    }

    // Large test graph schema:
    //
    //     City(1) ──Railway──> City(2) ──Railway──> City(3) ──Railway──> City(4)
    //       │                      │                    │                    │
    //       │                      │                    │                    │
    //       │                      └──Highway──> Town(5) │                    │
    //       │                                           │                    │
    //       └──Highway──> Town(6)                      │                    │
    //                                                      │                    │
    //     City(7) ──Railway──> City(8) ──Highway──> Town(9) ──Highway──> Town(10)
    //       │                      │
    //       │                      │
    //       └──Railway──> City(2) ──┘
    //
    //     Town(11) ──Highway──> Town(12) ──Highway──> Town(13)
    //       │
    //       └──Highway──> City(1)
    //
    fn create_large_test_graph() -> GraphStore {
        let authority = Pubkey::new_unique();

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        nodes.push(Node {
            id: 1,
            label: "City".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![0, 1],
        });

        nodes.push(Node {
            id: 2,
            label: "City".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![2, 3],
        });

        nodes.push(Node {
            id: 3,
            label: "City".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![4],
        });

        nodes.push(Node {
            id: 4,
            label: "City".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![],
        });

        nodes.push(Node {
            id: 5,
            label: "Town".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![],
        });

        nodes.push(Node {
            id: 6,
            label: "Town".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![],
        });

        nodes.push(Node {
            id: 7,
            label: "City".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![5, 6],
        });

        nodes.push(Node {
            id: 8,
            label: "City".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![7],
        });

        nodes.push(Node {
            id: 9,
            label: "Town".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![8],
        });

        nodes.push(Node {
            id: 10,
            label: "Town".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![],
        });

        nodes.push(Node {
            id: 11,
            label: "Town".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![9, 10],
        });

        nodes.push(Node {
            id: 12,
            label: "Town".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![11],
        });

        nodes.push(Node {
            id: 13,
            label: "Town".to_string(),
            data: Vec::new(),
            outgoing_edge_indices: vec![],
        });

        edges.push(Edge {
            from: 1,
            to: 2,
            label: "Railway".to_string(),
        });

        edges.push(Edge {
            from: 1,
            to: 6,
            label: "Highway".to_string(),
        });

        edges.push(Edge {
            from: 2,
            to: 3,
            label: "Railway".to_string(),
        });

        edges.push(Edge {
            from: 2,
            to: 5,
            label: "Highway".to_string(),
        });

        edges.push(Edge {
            from: 3,
            to: 4,
            label: "Railway".to_string(),
        });

        edges.push(Edge {
            from: 7,
            to: 2,
            label: "Railway".to_string(),
        });

        edges.push(Edge {
            from: 7,
            to: 8,
            label: "Railway".to_string(),
        });

        edges.push(Edge {
            from: 8,
            to: 9,
            label: "Highway".to_string(),
        });

        edges.push(Edge {
            from: 9,
            to: 10,
            label: "Highway".to_string(),
        });

        edges.push(Edge {
            from: 11,
            to: 1,
            label: "Highway".to_string(),
        });

        edges.push(Edge {
            from: 11,
            to: 12,
            label: "Highway".to_string(),
        });

        edges.push(Edge {
            from: 12,
            to: 13,
            label: "Highway".to_string(),
        });

        GraphStore {
            authority,
            node_count: 13,
            edge_count: 12,
            nonce: 14,
            nodes,
            edges,
        }
    }

    #[test]
    fn test_traverse_out_large_graph_simple_railway() {
        let graph = create_large_test_graph();

        let filter = create_filter("City", "Railway");
        let result = graph.traverse_out(&[1], &filter, None);

        assert_eq!(result.len(), 4);
        assert!(result.contains(&1)); // Start node is included
        assert!(result.contains(&2));
        assert!(result.contains(&3));
        assert!(result.contains(&4));
    }

    #[test]
    fn test_traverse_out_large_graph_simple_highway() {
        let graph = create_large_test_graph();

        let filter = create_filter("Town", "Highway");
        let result = graph.traverse_out(&[11], &filter, None);

        assert_eq!(result.len(), 3);
        assert!(result.contains(&12));
        assert!(result.contains(&13));
        assert!(result.contains(&11));
    }
}
