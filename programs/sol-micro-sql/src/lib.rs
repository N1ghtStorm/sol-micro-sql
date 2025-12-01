mod cypher;
mod graph;
mod lexer;
mod vm;

use crate::graph::GraphStore;
use anchor_lang::prelude::*;

declare_id!("9jJqjrdiJTYo9vYftpxJoLrLeuBn2qEQEX8Au1P8r1Gj");

#[program]
pub mod sol_micro_sql {
    use super::*;

    pub fn initialize_graph(ctx: Context<InitializeGraph>) -> Result<()> {
        let graph = &mut ctx.accounts.graph_store;
        graph.authority = ctx.accounts.authority.key();
        graph.node_count = 0;
        graph.edge_count = 0;
        graph.nonce = 0;
        graph.nodes = Vec::new();
        graph.edges = Vec::new();

        msg!(
            "GraphStore initialized by: {:?}",
            ctx.accounts.authority.key()
        );
        Ok(())
    }

    pub fn get_node_info(ctx: Context<GetNodeInfo>, node_id: u128) -> Result<()> {
        let graph = &ctx.accounts.graph_store;

        let node = graph
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .ok_or(ErrorCode::NodeNotFound)?;

        msg!(
            "Node {}: label='{}', outgoing_edges={}",
            node_id,
            node.label,
            node.outgoing_edge_indices.len()
        );

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeGraph<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 8 + 8 + 16 + 4 + (500 * 1024) + 4 + (200 * 1024),
        seeds = [b"graph_store"],
        bump
    )]
    pub graph_store: Account<'info, GraphStore>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetNodeInfo<'info> {
    #[account(
        seeds = [b"graph_store"],
        bump
    )]
    pub graph_store: Account<'info, GraphStore>,
}

#[event]
pub struct NodeAdded {
    pub node_id: u128,
    pub node_count: u64,
}

#[event]
pub struct EdgeAdded {
    pub from: u128,
    pub to: u128,
    pub edge_count: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Node not found")]
    NodeNotFound,
    #[msg("Duplicate node ID")]
    DuplicateNodeId,
    #[msg("Overflow")]
    Overflow,
}
