use crate::graph::{NodeId, GraphStore as Graph};

#[derive(Debug, Clone)]
pub enum Opcode {
    TraverseOut{
        node_label: String,
        edge_label: String,
    },
    TraverseIn{
        node_label: String,
        edge_label: String,
    },
    FilterNodeLabel{
        node_label: String,
    },
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
    stack: Vec<VmValue>,
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
            stack: Vec::new(),
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
                Opcode::TraverseOut { node_label, edge_label } => {
                    let start_nodes = self.get_current_nodes()?;
                    let result = self.graph.traverse_out(
                        start_nodes,
                        node_label,
                        edge_label,
                        self.limit,
                    );
                    self.current_set = result;
                }
                Opcode::TraverseIn { node_label, edge_label } => {
                    let start_nodes = self.get_current_nodes()?;
                    let result = self.traverse_in(start_nodes, node_label, edge_label)?;
                    self.current_set = result;
                }
                Opcode::FilterNodeLabel { node_label } => {
                    self.current_set.retain(|&node_id| {
                        self.graph
                            .get_node_by_id(node_id)
                            .map(|n| n.label == *node_label)
                            .unwrap_or(false)
                    });
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

    fn traverse_in(
        &self,
        start_nodes: &[NodeId],
        node_label: &str,
        edge_label: &str,
    ) -> Result<Vec<NodeId>, VmError> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        for &node_id in start_nodes {
            if let Some(node) = self.graph.get_node_by_id(node_id) {
                if node.label == node_label {
                    queue.push_back(node_id);
                    visited.insert(node_id);
                }
            }
        }

        while let Some(current_id) = queue.pop_front() {
            if let Some(limit) = self.limit {
                if result.len() >= limit {
                    break;
                }
            }

            for edge in &self.graph.edges {
                if edge.to == current_id && edge.label == edge_label {
                    let source_id = edge.from;
                    
                    if !visited.contains(&source_id) {
                        if let Some(source_node) = self.graph.get_node_by_id(source_id) {
                            if source_node.label == node_label {
                                visited.insert(source_id);
                                result.push(source_id);
                                
                                if let Some(limit) = self.limit {
                                    if result.len() >= limit {
                                        return Ok(result);
                                    }
                                }
                                
                                queue.push_back(source_id);
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn set_current_set(&mut self, nodes: Vec<NodeId>) {
        self.current_set = nodes;
    }
}

