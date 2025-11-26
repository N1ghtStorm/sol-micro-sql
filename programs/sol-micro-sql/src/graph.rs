use anchor_lang::prelude::*;

pub type NodeId = u128;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub attributes: Vec<(String, String)>,
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
        node_label: &str,
        edge_label: &str,
        limit: Option<usize>,
    ) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        for &node_id in start_nodes {
            if self.get_node_by_id(node_id).is_some() {
                queue.push_back(node_id);
                visited.insert(node_id);
            }
        }

        while let Some(current_id) = queue.pop_front() {
            if let Some(limit) = limit {
                if result.len() >= limit {
                    break;
                }
            }

            if let Some(current_node) = self.get_node_by_id(current_id) {
                for &edge_index in &current_node.outgoing_edge_indices {
                    if let Some(edge) = self.edges.get(edge_index as usize) {
                        if edge.label == edge_label {
                            let target_id = edge.to;
                            
                            if !visited.contains(&target_id) {
                                visited.insert(target_id);
                                
                                if let Some(target_node) = self.get_node_by_id(target_id) {
                                    if target_node.label == node_label {
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

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

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
            attributes: Vec::new(),
            outgoing_edge_indices: vec![0, 1],
        });

        nodes.push(Node {
            id: 2,
            label: "City".to_string(),
            attributes: Vec::new(),
            outgoing_edge_indices: vec![2, 3],
        });

        nodes.push(Node {
            id: 3,
            label: "City".to_string(),
            attributes: Vec::new(),
            outgoing_edge_indices: vec![4],
        });

        nodes.push(Node {
            id: 4,
            label: "Town".to_string(),
            attributes: Vec::new(),
            outgoing_edge_indices: vec![],
        });

        nodes.push(Node {
            id: 5,
            label: "Town".to_string(),
            attributes: Vec::new(),
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
        
        let result = graph.traverse_out(&[1], "City", "Railway", None);
        
        assert_eq!(result.len(), 2);
        assert!(result.contains(&2));
        assert!(result.contains(&3));
    }

    #[test]
    fn test_traverse_out_with_limit() {
        let graph = create_small_test_graph();
        
        let result = graph.traverse_out(&[1], "City", "Railway", Some(1));
        
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_traverse_out_wrong_edge_label() {
        let graph = create_small_test_graph();
        
        let result = graph.traverse_out(&[1], "City", "NONEXISTENT", None);
        
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_traverse_out_wrong_node_label() {
        let graph = create_small_test_graph();
        
        let result = graph.traverse_out(&[1], "Town", "Railway", None);
        
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_traverse_out_multiple_start_nodes() {
        let graph = create_small_test_graph();
        
        let result = graph.traverse_out(&[1, 2], "City", "Railway", None);
        
        assert_eq!(result.len(), 1);
        assert!(result.contains(&3));
    }

    #[test]
    fn test_traverse_out_handles_cycles() {
        let graph = create_small_test_graph();
        
        let result = graph.traverse_out(&[1], "City", "Railway", None);
        
        assert_eq!(result.len(), 2);
        assert!(!result.contains(&1));
        assert!(result.contains(&2));
        assert!(result.contains(&3));
    }

    #[test]
    fn test_traverse_out_different_edge_types() {
        let graph = create_small_test_graph();
        
        let result = graph.traverse_out(&[2], "Town", "Highway", None);
        
        assert_eq!(result.len(), 1);
        assert!(result.contains(&4));
    }

    #[test]
    fn test_traverse_out_nonexistent_start_node() {
        let graph = create_small_test_graph();
        
        let result = graph.traverse_out(&[999], "City", "Railway", None);
        
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_traverse_out_empty_start_nodes() {
        let graph = create_small_test_graph();
        
        let result = graph.traverse_out(&[], "City", "Railway", None);
        
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_traverse_out_multi_hop() {
        let graph = create_small_test_graph();
        
        let result = graph.traverse_out(&[1], "City", "Railway", None);
        
        assert_eq!(result.len(), 2);
        assert!(result.contains(&2));
        assert!(result.contains(&3));
    }
}