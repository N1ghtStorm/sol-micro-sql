use crate::graph::{NodeId, GraphStore as Graph, TraverseFilter, Node, Edge};

#[derive(Debug, Clone)]
pub enum Opcode {
    SetCurrentFromAllNodes,
    SetCurrentFromIds(Vec<NodeId>),
    TraverseOut(TraverseFilter),
    SetLimit(usize),
    SaveResults,
    CreateNode { label: String, attributes: Vec<(String, String)> },
    CreateEdge { from: NodeId, to: NodeId, label: String },
}

#[derive(Debug, Clone)]
pub enum VmResult {
    Nodes(Vec<NodeId>),
    Scalar(i64),
    None,
}

#[derive(Debug, Clone)]
pub enum VmValue {
    Int(i64),
    Str(String),
}

pub struct Vm<'g> {
    graph: &'g mut Graph,
    current_set: Vec<NodeId>,
    result_set: Vec<NodeId>,
    limit: Option<usize>,
}

#[derive(Debug)]
pub enum VmError {
    NoReturnValue,
    StackUnderflow,
    InvalidNodeSet,
    NodeNotFound,
    Overflow,
}

impl<'g> Vm<'g> {
    pub fn new(graph: &'g mut Graph) -> Self {
        Self {
            graph,
            current_set: Vec::new(),
            result_set: Vec::new(),
            limit: None,
        }
    }

    fn get_current_nodes(&self) -> Result<&[NodeId], VmError> {
        if self.current_set.is_empty() {
            return Err(VmError::InvalidNodeSet);
        }
        Ok(&self.current_set)
    }

    pub fn execute(&mut self, ops: &[Opcode]) -> Result<VmResult, VmError> {
        for op in ops {
            match op {
                Opcode::SetCurrentFromAllNodes => {
                    self.current_set = self.graph.nodes.iter().map(|n| n.id).collect();
                }
                Opcode::SetCurrentFromIds(node_ids) => {
                    self.current_set = node_ids.clone();
                }
                Opcode::TraverseOut(filter) => {
                    let start_nodes = self.get_current_nodes()?;
                    let result = self.graph.traverse_out(
                        start_nodes,
                        filter,
                        self.limit,
                    );
                    self.current_set = result;
                }
                Opcode::SetLimit(limit) => {
                    self.limit = Some(*limit);
                }
                Opcode::SaveResults => {
                    self.result_set.extend_from_slice(&self.current_set);
                }
                Opcode::CreateNode { label, attributes } => {
                    let id = self.graph.nonce;
                    self.graph.nonce = self.graph.nonce
                        .checked_add(1)
                        .ok_or(VmError::Overflow)?;

                    let node = Node {
                        id,
                        label: label.clone(),
                        attributes: attributes.clone(),
                        outgoing_edge_indices: Vec::new(),
                    };

                    self.graph.nodes.push(node);
                    self.graph.node_count = self.graph.node_count
                        .checked_add(1)
                        .ok_or(VmError::Overflow)?;
                    
                    // Set the created node as the current set
                    self.current_set = vec![id];
                }
                Opcode::CreateEdge { from, to, label } => {
                    let from_exists = self.graph.nodes.iter().any(|n| n.id == *from);
                    let to_exists = self.graph.nodes.iter().any(|n| n.id == *to);
                    
                    if !from_exists || !to_exists {
                        return Err(VmError::NodeNotFound);
                    }

                    let edge_index = self.graph.edges.len() as u32;
                    let edge = Edge {
                        from: *from,
                        to: *to,
                        label: label.clone(),
                    };

                    self.graph.edges.push(edge);
                    self.graph.edge_count = self.graph.edge_count
                        .checked_add(1)
                        .ok_or(VmError::Overflow)?;

                    let from_node = self.graph.nodes
                        .iter_mut()
                        .find(|n| n.id == *from)
                        .ok_or(VmError::NodeNotFound)?;
                    
                    from_node.outgoing_edge_indices.push(edge_index);
                    
                    // Set the current set to the "to" node
                    self.current_set = vec![*to];
                }
            }
        }

        if !self.current_set.is_empty() {
            Ok(VmResult::Nodes(self.current_set.clone()))
        } else if !self.result_set.is_empty() {
            Ok(VmResult::Nodes(self.result_set.clone()))
        } else {
            Err(VmError::NoReturnValue)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphStore, Node, Edge};
    use anchor_lang::prelude::Pubkey;

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

    fn create_filter(node_label: &str, edge_label: &str) -> TraverseFilter {
        TraverseFilter {
            where_node_labels: vec![node_label.to_string()],
            where_edge_labels: vec![edge_label.to_string()],
            where_not_node_labels: Vec::new(),
            where_not_edge_labels: Vec::new(),
        }
    }

    #[test]
    fn test_set_current_from_all_nodes() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let ops = vec![Opcode::SetCurrentFromAllNodes];
        let result = vm.execute(&ops).unwrap();
        
        match result {
            VmResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 5);
                assert!(nodes.contains(&1));
                assert!(nodes.contains(&2));
                assert!(nodes.contains(&3));
                assert!(nodes.contains(&4));
                assert!(nodes.contains(&5));
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_set_current_from_ids() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let ops = vec![Opcode::SetCurrentFromIds(vec![1, 3, 5])];
        let result = vm.execute(&ops).unwrap();
        
        match result {
            VmResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 3);
                assert!(nodes.contains(&1));
                assert!(nodes.contains(&3));
                assert!(nodes.contains(&5));
                assert!(!nodes.contains(&2));
                assert!(!nodes.contains(&4));
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_filter_node_label_via_traverse() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let filter = TraverseFilter {
            where_node_labels: vec!["City".to_string()],
            where_edge_labels: Vec::new(),
            where_not_node_labels: Vec::new(),
            where_not_edge_labels: Vec::new(),
        };
        let ops = vec![
            Opcode::SetCurrentFromAllNodes,
            Opcode::TraverseOut(filter),
        ];
        let result = vm.execute(&ops).unwrap();
        
        match result {
            VmResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 3);
                assert!(nodes.contains(&1));
                assert!(nodes.contains(&2));
                assert!(nodes.contains(&3));
                assert!(!nodes.contains(&4));
                assert!(!nodes.contains(&5));
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_filter_node_label_not_via_traverse() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let filter = TraverseFilter {
            where_node_labels: Vec::new(),
            where_edge_labels: Vec::new(),
            where_not_node_labels: vec!["Town".to_string()],
            where_not_edge_labels: Vec::new(),
        };
        let ops = vec![
            Opcode::SetCurrentFromAllNodes,
            Opcode::TraverseOut(filter),
        ];
        let result = vm.execute(&ops).unwrap();
        
        match result {
            VmResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 3);
                assert!(nodes.contains(&1));
                assert!(nodes.contains(&2));
                assert!(nodes.contains(&3));
                assert!(!nodes.contains(&4));
                assert!(!nodes.contains(&5));
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_traverse_out() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let filter = create_filter("City", "Railway");
        let ops = vec![
            Opcode::SetCurrentFromIds(vec![1]),
            Opcode::TraverseOut(filter),
        ];
        let result = vm.execute(&ops).unwrap();
        
        match result {
            VmResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 3);
                assert!(nodes.contains(&1));
                assert!(nodes.contains(&2));
                assert!(nodes.contains(&3));
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_traverse_out_with_limit() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let filter = create_filter("City", "Railway");
        let ops = vec![
            Opcode::SetCurrentFromIds(vec![1]),
            Opcode::SetLimit(2),
            Opcode::TraverseOut(filter),
        ];
        let result = vm.execute(&ops).unwrap();
        
        match result {
            VmResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 2);
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_save_results() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let ops = vec![
            Opcode::SetCurrentFromIds(vec![1, 2]),
            Opcode::SaveResults,
            Opcode::SetCurrentFromIds(vec![]),
        ];
        let result = vm.execute(&ops).unwrap();
        
        match result {
            VmResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 2);
                assert!(nodes.contains(&1));
                assert!(nodes.contains(&2));
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_complex_query() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let filter1 = TraverseFilter {
            where_node_labels: vec!["City".to_string()],
            where_edge_labels: Vec::new(),
            where_not_node_labels: Vec::new(),
            where_not_edge_labels: Vec::new(),
        };
        
        let filter2 = create_filter("City", "Railway");
        let ops = vec![
            Opcode::SetCurrentFromAllNodes,
            Opcode::TraverseOut(filter1),
            Opcode::SetCurrentFromIds(vec![1]),
            Opcode::TraverseOut(filter2),
        ];
        let result = vm.execute(&ops).unwrap();
        
        match result {
            VmResult::Nodes(nodes) => {
                assert!(nodes.len() >= 2);
                assert!(nodes.contains(&1));
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_traverse_out_empty_current_set() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let filter = create_filter("City", "Railway");
        let ops = vec![Opcode::TraverseOut(filter)];
        let result = vm.execute(&ops);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            VmError::InvalidNodeSet => {}
            _ => panic!("Expected InvalidNodeSet error"),
        }
    }

    #[test]
    fn test_no_return_value() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let filter = TraverseFilter {
            where_node_labels: vec!["NonExistent".to_string()],
            where_edge_labels: Vec::new(),
            where_not_node_labels: Vec::new(),
            where_not_edge_labels: Vec::new(),
        };
        let ops = vec![
            Opcode::SetCurrentFromIds(vec![1, 2, 3]),
            Opcode::TraverseOut(filter),
        ];
        let result = vm.execute(&ops);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            VmError::NoReturnValue => {}
            _ => panic!("Expected NoReturnValue error"),
        }
    }

    #[test]
    fn test_filter_after_traverse() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let filter1 = create_filter("City", "Railway");
        let filter2 = TraverseFilter {
            where_node_labels: vec!["City".to_string()],
            where_edge_labels: Vec::new(),
            where_not_node_labels: Vec::new(),
            where_not_edge_labels: Vec::new(),
        };
        let ops = vec![
            Opcode::SetCurrentFromIds(vec![1]),
            Opcode::TraverseOut(filter1),
            Opcode::TraverseOut(filter2),
        ];
        let result = vm.execute(&ops).unwrap();
        
        // Drop VM to release mutable borrow before accessing graph
        drop(vm);
        
        match result {
            VmResult::Nodes(nodes) => {
                assert!(nodes.len() >= 2);
                for &node_id in &nodes {
                    let node = graph.get_node_by_id(node_id).unwrap();
                    assert_eq!(node.label, "City");
                }
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_multiple_traversals() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let filter1 = create_filter("City", "Railway");
        let filter2 = create_filter("Town", "Highway");
        let ops = vec![
            Opcode::SetCurrentFromIds(vec![2]),
            Opcode::TraverseOut(filter2),
            Opcode::SetCurrentFromIds(vec![1]),
            Opcode::TraverseOut(filter1),
        ];
        let result = vm.execute(&ops).unwrap();
        
        match result {
            VmResult::Nodes(nodes) => {
                assert!(nodes.len() >= 2);
                assert!(nodes.contains(&1));
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_create_node() {
        let mut graph = create_small_test_graph();
        let initial_node_count = graph.node_count;
        let initial_nonce = graph.nonce;
        
        let mut vm = Vm::new(&mut graph);
        
        let ops = vec![Opcode::CreateNode {
            label: "Village".to_string(),
            attributes: vec![("population".to_string(), "1000".to_string())],
        }];
        let result = vm.execute(&ops).unwrap();
        
        drop(vm);
        
        // Check that node was created
        assert_eq!(graph.node_count, initial_node_count + 1);
        assert_eq!(graph.nonce, initial_nonce + 1);
        
        // Check result contains the new node ID
        match result {
            VmResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 1);
                let new_node_id = nodes[0];
                assert_eq!(new_node_id, initial_nonce);
                
                // Verify the node exists in the graph
                let node = graph.get_node_by_id(new_node_id).unwrap();
                assert_eq!(node.label, "Village");
                assert_eq!(node.attributes.len(), 1);
                assert_eq!(node.attributes[0].0, "population");
                assert_eq!(node.attributes[0].1, "1000");
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_create_edge() {
        let mut graph = create_small_test_graph();
        let initial_edge_count = graph.edge_count;
        
        let mut vm = Vm::new(&mut graph);
        
        let ops = vec![Opcode::CreateEdge {
            from: 1,
            to: 5,
            label: "Road".to_string(),
        }];
        let result = vm.execute(&ops);
        
        drop(vm);
        
        // Check that edge was created
        assert!(result.is_ok());
        assert_eq!(graph.edge_count, initial_edge_count + 1);
        
        // Verify the edge exists and is linked from node 1
        let node1 = graph.get_node_by_id(1).unwrap();
        assert!(node1.outgoing_edge_indices.len() > 0);
        
        let last_edge_index = node1.outgoing_edge_indices.last().unwrap();
        let edge = &graph.edges[*last_edge_index as usize];
        assert_eq!(edge.from, 1);
        assert_eq!(edge.to, 5);
        assert_eq!(edge.label, "Road");
    }

    #[test]
    fn test_create_edge_invalid_from_node() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let ops = vec![Opcode::CreateEdge {
            from: 999, // Non-existent node
            to: 1,
            label: "Road".to_string(),
        }];
        let result = vm.execute(&ops);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            VmError::NodeNotFound => {}
            _ => panic!("Expected NodeNotFound error"),
        }
    }

    #[test]
    fn test_create_edge_invalid_to_node() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        let ops = vec![Opcode::CreateEdge {
            from: 1,
            to: 999, // Non-existent node
            label: "Road".to_string(),
        }];
        let result = vm.execute(&ops);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            VmError::NodeNotFound => {}
            _ => panic!("Expected NodeNotFound error"),
        }
    }

    #[test]
    fn test_create_node_and_edge_sequence() {
        let mut graph = create_small_test_graph();
        let mut vm = Vm::new(&mut graph);
        
        // Create a new node
        let ops1 = vec![Opcode::CreateNode {
            label: "Village".to_string(),
            attributes: Vec::new(),
        }];
        let result1 = vm.execute(&ops1).unwrap();
        
        let new_node_id = match result1 {
            VmResult::Nodes(nodes) => nodes[0],
            _ => panic!("Expected Nodes result"),
        };
        
        // Create an edge from existing node to the new node
        let ops2 = vec![Opcode::CreateEdge {
            from: 1,
            to: new_node_id,
            label: "Path".to_string(),
        }];
        let result2 = vm.execute(&ops2);
        
        drop(vm);
        
        assert!(result2.is_ok());
        
        // Verify both node and edge exist
        let node = graph.get_node_by_id(new_node_id);
        assert!(node.is_some());
        assert_eq!(node.unwrap().label, "Village");
        
        let node1 = graph.get_node_by_id(1).unwrap();
        let last_edge_index = node1.outgoing_edge_indices.last().unwrap();
        let edge = &graph.edges[*last_edge_index as usize];
        assert_eq!(edge.to, new_node_id);
        assert_eq!(edge.label, "Path");
    }
}

