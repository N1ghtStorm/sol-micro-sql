use crate::graph::{NodeId, GraphStore as Graph};

#[derive(Debug, Clone)]
pub enum Opcode {
    TraverseOut{
        where_node_label: String,
        where_edge_label: String,
        where_not_node_label: String,
        where_not_edge_label: String,
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
                // Opcode::TraverseOut { where_node_label: node_label, where_edge_label: edge_label } => {
                //     let start_nodes = self.get_current_nodes()?;
                //     let result = self.graph.traverse_out(
                //         start_nodes,
                //         node_label,
                //         edge_label,
                //         self.limit,
                //     );
                //     self.current_set = result;
                // }
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
                _ => continue,
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

