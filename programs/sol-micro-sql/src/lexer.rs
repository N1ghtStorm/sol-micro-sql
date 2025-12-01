use crate::graph::TraverseFilter;
use crate::vm::Opcode;
use crate::cypher::{CypherQuery, MatchPattern, WhereClause, CreatePattern};

pub fn compile_to_opcodes(query: CypherQuery) -> Vec<Opcode> {
    let mut opcodes = Vec::new();
    
    match query {
        CypherQuery::Match { match_pattern, where_clause, limit, .. } => {
            match match_pattern {
                MatchPattern::SingleNode { variable: _, label } => {
                    opcodes.push(Opcode::SetCurrentFromAllNodes);
                    
                    if let Some(label) = label {
                        let filter = TraverseFilter {
                            where_node_labels: vec![label],
                            where_edge_labels: Vec::new(),
                            where_not_node_labels: Vec::new(),
                            where_not_edge_labels: Vec::new(),
                        };
                        opcodes.push(Opcode::TraverseOut(filter));
                    }
                }
                MatchPattern::Relationship { from, edge, to } => {
                    if let Some(start_id) = extract_start_node_id(&where_clause) {
                        opcodes.push(Opcode::SetCurrentFromIds(vec![start_id]));
                    } else {
                        opcodes.push(Opcode::SetCurrentFromAllNodes);
                        
                        if let Some(label) = &from.label {
                            let filter = TraverseFilter {
                                where_node_labels: vec![label.clone()],
                                where_edge_labels: Vec::new(),
                                where_not_node_labels: Vec::new(),
                                where_not_edge_labels: Vec::new(),
                            };
                            opcodes.push(Opcode::TraverseOut(filter));
                        }
                    }
                    
                    if let Some(edge_label) = edge.label {
                        let filter = TraverseFilter {
                            where_node_labels: to.label.map(|l| vec![l]).unwrap_or_default(),
                            where_edge_labels: vec![edge_label],
                            where_not_node_labels: Vec::new(),
                            where_not_edge_labels: Vec::new(),
                        };
                        opcodes.push(Opcode::TraverseOut(filter));
                    }
                }
            }
            
            if let Some(limit) = limit {
                opcodes.push(Opcode::SetLimit(limit));
            }
            
            opcodes.push(Opcode::SaveResults);
        }
        CypherQuery::Create { create_pattern } => {
            match create_pattern {
                CreatePattern::Node { label, .. } => {
                    opcodes.push(Opcode::CreateNode {
                        label: label.unwrap_or_default(),
                        data: Vec::new(),
                    });
                }
                CreatePattern::Edge { from_id, to_id, edge, .. } => {
                    // For CREATE edge, use the node IDs if provided directly
                    // For MVP, we require explicit node IDs (numeric)
                    // Variable resolution can be added in the future
                    if let (Some(from), Some(to)) = (from_id, to_id) {
                        let edge_label = edge.label.unwrap_or_default();
                        opcodes.push(Opcode::CreateEdge {
                            from,
                            to,
                            label: edge_label,
                        });
                    }
                    // If node IDs are not provided, skip edge creation
                    // In a full implementation, you'd resolve variables here
                }
            }
        }
    }
    
    opcodes
}

fn extract_start_node_id(where_clause: &Option<WhereClause>) -> Option<u128> {
    if let Some(WhereClause::NodeIdEq { value, .. }) = where_clause {
        Some(*value)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cypher::{CypherQuery, MatchPattern, NodePattern, EdgePattern, EdgeDirection, WhereClause, ReturnClause};

    #[test]
    fn test_compile_relationship_query() {
        let query = CypherQuery::Match {
            match_pattern: MatchPattern::Relationship {
                from: NodePattern {
                    variable: "n".to_string(),
                    label: Some("User".to_string()),
                },
                edge: EdgePattern {
                    direction: EdgeDirection::Outgoing,
                    label: Some("FOLLOWS".to_string()),
                },
                to: NodePattern {
                    variable: "m".to_string(),
                    label: Some("User".to_string()),
                },
            },
            where_clause: Some(WhereClause::NodeIdEq {
                variable: "n".to_string(),
                value: 42,
            }),
            return_clause: ReturnClause::NodeId { variable: "m".to_string() },
            limit: Some(10),
        };
        
        let opcodes = compile_to_opcodes(query);
        assert!(opcodes.len() >= 3);
    }

    #[test]
    fn test_compile_with_start_node_id() {
        let query = CypherQuery::Match {
            match_pattern: MatchPattern::Relationship {
                from: NodePattern {
                    variable: "n".to_string(),
                    label: Some("User".to_string()),
                },
                edge: EdgePattern {
                    direction: EdgeDirection::Outgoing,
                    label: Some("FOLLOWS".to_string()),
                },
                to: NodePattern {
                    variable: "m".to_string(),
                    label: Some("User".to_string()),
                },
            },
            where_clause: Some(WhereClause::NodeIdEq {
                variable: "n".to_string(),
                value: 42,
            }),
            return_clause: ReturnClause::NodeId { variable: "m".to_string() },
            limit: Some(10),
        };
        
        let opcodes = compile_to_opcodes(query);
        assert!(opcodes.len() >= 3);
        
        match &opcodes[0] {
            Opcode::SetCurrentFromIds(ids) => {
                assert_eq!(ids, &vec![42]);
            }
            _ => panic!("Expected SetCurrentFromIds with start node id"),
        }
    }
}

