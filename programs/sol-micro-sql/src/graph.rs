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