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