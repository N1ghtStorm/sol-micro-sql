use crate::graph::{NodeId, GraphStore as Graph, TraverseFilter};

#[derive(Debug, Clone)]
pub enum Opcode {
    SetCurrentFromAllNodes,
    SetCurrentFromIds(Vec<NodeId>),
    TraverseOut(TraverseFilter),
    SetLimit(usize),
    SaveResults,
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
    graph: &'g Graph,
    current_set: Vec<NodeId>,
    result_set: Vec<NodeId>,
    limit: Option<usize>,
}

#[derive(Debug)]
pub enum VmError {
    NoReturnValue,
    StackUnderflow,
    InvalidNodeSet,
}

impl<'g> Vm<'g> {
    pub fn new(graph: &'g Graph) -> Self {
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
        let graph = create_small_test_graph();
        let mut vm = Vm::new(&graph);
        
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
}

