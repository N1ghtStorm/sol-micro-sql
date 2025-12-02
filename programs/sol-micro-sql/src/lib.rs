mod cypher;
mod graph;
mod lexer;
mod vm;

use crate::cypher::{parse, CypherQuery};
use crate::graph::GraphStore;
use crate::lexer::compile_to_opcodes;
use crate::vm::{Vm, VmError, VmResult};
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

    pub fn execute_query(ctx: Context<ExecuteQuery>, query: String) -> Result<VmResult> {
        let graph = &ctx.accounts.graph_store;
        let cypher_query = parse(&query).map_err(|_| ErrorCode::QueryExecutionFailed)?;

        let has_create = matches!(cypher_query, CypherQuery::Create { .. });

        if has_create {
            require!(
                ctx.accounts.authority.key() == graph.authority,
                ErrorCode::Unauthorized
            );
        }

        let graph = &mut ctx.accounts.graph_store;
        let ops = compile_to_opcodes(cypher_query);

        require!(query.len() <= 4096, ErrorCode::QueryExecutionFailed);
        require!(ops.len() <= 100, ErrorCode::QueryExecutionFailed);

        let mut vm = Vm::new(graph);
        let result = vm.execute(&ops).map_err(|e| match e {
            VmError::NodeNotFound => ErrorCode::NodeNotFound,
            VmError::Overflow => ErrorCode::Overflow,
            VmError::DataTooLarge | VmError::LabelTooLong | VmError::GraphLimitExceeded => {
                ErrorCode::QueryExecutionFailed
            }
            _ => ErrorCode::QueryExecutionFailed,
        })?;
        Ok(result)
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
        space = 8 +
                32 +
                8 +
                8 +
                16 +
                4 + (512) +
                4 + (256),
        seeds = [b"graph_store"],
        bump
    )]
    pub graph_store: Account<'info, GraphStore>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteQuery<'info> {
    #[account(
        mut,
        seeds = [b"graph_store"],
        bump
    )]
    pub graph_store: Account<'info, GraphStore>,

    /// CHECK: Authority is only required for CREATE operations, checked in the function
    pub authority: UncheckedAccount<'info>,
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
    #[msg("Query execution failed")]
    QueryExecutionFailed,
    #[msg("Data too large")]
    DataTooLarge,
    #[msg("Label too long")]
    LabelTooLong,
    #[msg("Graph limit exceeded")]
    GraphLimitExceeded,
}
