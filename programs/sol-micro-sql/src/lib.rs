mod vm;
mod graph;

use anchor_lang::prelude::*;
use crate::graph::GraphStore;

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
        
        msg!("GraphStore initialized by: {:?}", ctx.accounts.authority.key());
        Ok(())
    }

    #[cfg(feature = "testnet")]
    pub fn add_node(
        ctx: Context<AddNode>,
        label: String,
    ) -> Result<u128> {
        let graph = &mut ctx.accounts.graph_store;
        
        let id = graph.nonce;
        graph.nonce = graph.nonce
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        let node = Node {
            id,
            label,
            attributes: Vec::new(),
            outgoing_edge_indices: Vec::new(),
        };

        graph.nodes.push(node);
        graph.node_count = graph.node_count
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        msg!("Added node with id: {}, total nodes: {}", id, graph.node_count);
        emit!(NodeAdded {
            node_id: id,
            node_count: graph.node_count,
        });
        
        Ok(id)
    }

    #[cfg(feature = "testnet")]
    pub fn set_node_attribute(
        ctx: Context<SetNodeAttribute>,
        node_id: u128,
        key: String,
        value: String,
    ) -> Result<()> {
        let graph = &mut ctx.accounts.graph_store;
        
        let node = graph.nodes
            .iter_mut()
            .find(|n| n.id == node_id)
            .ok_or(ErrorCode::NodeNotFound)?;

        if let Some(existing) = node.attributes.iter_mut().find(|(k, _)| k == &key) {
            existing.1 = value.clone();
        } else {
            node.attributes.push((key.clone(), value.clone()));
        }
        
        msg!("Set attribute '{}' = '{}' for node {}", key, value, node_id);
        Ok(())
    }

    #[cfg(feature = "testnet")]
    pub fn add_edge(
        ctx: Context<AddEdge>,
        from: u128,
        to: u128,
        label: String,
    ) -> Result<()> {
        let graph = &mut ctx.accounts.graph_store;

        let from_exists = graph.nodes.iter().any(|n| n.id == from);
        let to_exists = graph.nodes.iter().any(|n| n.id == to);
        
        if !from_exists {
            return Err(ErrorCode::NodeNotFound.into());
        }
        if !to_exists {
            return Err(ErrorCode::NodeNotFound.into());
        }

        let edge_index = graph.edges.len() as u32;
        let edge = Edge {
            from,
            to,
            label,
        };

        graph.edges.push(edge);
        graph.edge_count = graph.edge_count
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        let from_node = graph.nodes
            .iter_mut()
            .find(|n| n.id == from)
            .ok_or(ErrorCode::NodeNotFound)?;
        
        from_node.outgoing_edge_indices.push(edge_index);

        msg!("Added edge from {} to {} with label, total edges: {}", from, to, graph.edge_count);
        emit!(EdgeAdded {
            from,
            to,
            edge_count: graph.edge_count,
        });
        
        Ok(())
    }

    pub fn get_node_info(
        ctx: Context<GetNodeInfo>,
        node_id: u128,
    ) -> Result<()> {
        let graph = &ctx.accounts.graph_store;
        
        let node = graph.nodes
            .iter()
            .find(|n| n.id == node_id)
            .ok_or(ErrorCode::NodeNotFound)?;

        msg!("Node {}: label='{}', outgoing_edges={}", 
             node_id, node.label, node.outgoing_edge_indices.len());
        
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

#[cfg(feature = "testnet")]
#[derive(Accounts)]
pub struct AddNode<'info> {
    #[account(
        mut,
        seeds = [b"graph_store"],
        bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub graph_store: Account<'info, GraphStore>,
    
    pub authority: Signer<'info>,
}

#[cfg(feature = "testnet")]
#[derive(Accounts)]
pub struct SetNodeAttribute<'info> {
    #[account(
        mut,
        seeds = [b"graph_store"],
        bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub graph_store: Account<'info, GraphStore>,
    
    pub authority: Signer<'info>,
}

#[cfg(feature = "testnet")]
#[derive(Accounts)]
pub struct AddEdge<'info> {
    #[account(
        mut,
        seeds = [b"graph_store"],
        bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub graph_store: Account<'info, GraphStore>,
    
    pub authority: Signer<'info>,
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
