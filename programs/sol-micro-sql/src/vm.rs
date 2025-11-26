use crate::graph::{NodeId, GraphStore as Graph};

#[derive(Debug, Clone)]
pub enum Opcode {
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
}

#[derive(Debug)]
pub enum VmError {
    NoReturnValue
}

impl<'g> Vm<'g> {
    pub fn new(graph: &'g Graph) -> Self {
        Self {
            graph,
            stack: Vec::new(),
        }
    }

    pub fn execute(&mut self, opcode: &[Opcode]) -> Result<VmResult, VmError> {
        let mut last_result: Option<VmResult> = None;
        last_result.ok_or(VmError::NoReturnValue)
    }
}

