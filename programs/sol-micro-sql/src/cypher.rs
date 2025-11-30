use crate::graph::TraverseFilter;
use crate::vm::Opcode;

#[derive(Debug, Clone)]
pub struct CypherQuery {
    pub match_pattern: MatchPattern,
    pub where_clause: Option<WhereClause>,
    pub return_clause: ReturnClause,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum MatchPattern {
    SingleNode {
        variable: String,
        label: Option<String>,
    },
    Relationship {
        from: NodePattern,
        edge: EdgePattern,
        to: NodePattern,
    },
}

#[derive(Debug, Clone)]
pub struct NodePattern {
    pub variable: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EdgePattern {
    pub direction: EdgeDirection,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub enum EdgeDirection {
    Outgoing,
    Incoming,
    Bidirectional,
}

#[derive(Debug, Clone)]
pub enum WhereClause {
    NodeIdEq {
        variable: String,
        value: u128,
    },
    NodeAttrEq {
        variable: String,
        attr: String,
        value: String,
    },
}

#[derive(Debug, Clone)]
pub enum ReturnClause {
    NodeId {
        variable: String,
    },
    NodeAttr {
        variable: String,
        attr: String,
    },
    All,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(String),
    InvalidSyntax(String),
    MissingLimit,
}

pub fn parse(query: &str) -> Result<CypherQuery, ParseError> {
    let query = query.trim();
    let mut tokens = tokenize(query)?;
    
    let match_pattern = parse_match(&mut tokens)?;
    let where_clause = parse_where(&mut tokens)?;
    let return_clause = parse_return(&mut tokens)?;
    let limit = parse_limit(&mut tokens)?;
    
    if limit.is_none() {
        return Err(ParseError::MissingLimit);
    }
    
    if !tokens.is_empty() {
        return Err(ParseError::InvalidSyntax(format!("Unexpected tokens: {:?}", tokens)));
    }
    
    Ok(CypherQuery {
        match_pattern,
        where_clause,
        return_clause,
        limit,
    })
}

fn tokenize(input: &str) -> Result<Vec<String>, ParseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    
    for ch in input.chars() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                if in_string {
                    current.push(ch);
                } else if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            '(' | ')' | '[' | ']' | '-' | '>' | '<' | ':' | '=' | ',' => {
                if in_string {
                    current.push(ch);
                } else {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push(ch.to_string());
                }
            }
            '\'' | '"' => {
                if in_string {
                    tokens.push(current.clone());
                    current.clear();
                    in_string = false;
                } else {
                    in_string = true;
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    
    if !current.is_empty() {
        tokens.push(current);
    }
    
    Ok(tokens)
}

fn parse_match(tokens: &mut Vec<String>) -> Result<MatchPattern, ParseError> {
    expect_keyword(tokens, "MATCH")?;
    
    if tokens.is_empty() {
        return Err(ParseError::InvalidSyntax("Expected pattern after MATCH".to_string()));
    }
    
    let has_arrow = tokens.iter().any(|t| t == "->" || t == "<-" || t == "-");
    if has_arrow {
        parse_relationship_pattern(tokens)
    } else {
        parse_single_node_pattern(tokens)
    }
}

fn parse_single_node_pattern(tokens: &mut Vec<String>) -> Result<MatchPattern, ParseError> {
    expect_char(tokens, "(")?;
    
    let variable = expect_identifier(tokens)?;
    let label = if peek_token(tokens) == ":" {
        tokens.remove(0);
        Some(expect_identifier(tokens)?)
    } else {
        None
    };
    
    expect_char(tokens, ")")?;
    
    Ok(MatchPattern::SingleNode { variable, label })
}

fn parse_relationship_pattern(tokens: &mut Vec<String>) -> Result<MatchPattern, ParseError> {
    expect_char(tokens, "(")?;
    let from_var = expect_identifier(tokens)?;
    let from_label = if peek_token(tokens) == ":" {
        tokens.remove(0);
        Some(expect_identifier(tokens)?)
    } else {
        None
    };
    expect_char(tokens, ")")?;
    
    let direction = if peek_token(tokens) == "-" {
        tokens.remove(0);
        if peek_token(tokens) == ">" {
            tokens.remove(0);
            EdgeDirection::Outgoing
        } else if peek_token(tokens) == "<" {
            tokens.remove(0);
            EdgeDirection::Incoming
        } else {
            EdgeDirection::Bidirectional
        }
    } else {
        return Err(ParseError::InvalidSyntax("Expected edge pattern".to_string()));
    };
    
    expect_char(tokens, "[")?;
    let edge_label = if peek_token(tokens) == ":" {
        tokens.remove(0);
        if peek_token(tokens) == "]" {
            None
        } else {
            Some(expect_identifier(tokens)?)
        }
    } else {
        None
    };
    expect_char(tokens, "]")?;
    
    match direction {
        EdgeDirection::Outgoing => {
            if peek_token(tokens) == "-" {
                tokens.remove(0);
            }
            if peek_token(tokens) == ">" {
                tokens.remove(0);
            }
        }
        EdgeDirection::Incoming => {
            if peek_token(tokens) == "<" {
                tokens.remove(0);
            }
            if peek_token(tokens) == "-" {
                tokens.remove(0);
            }
        }
        EdgeDirection::Bidirectional => {
            if peek_token(tokens) == "-" {
                tokens.remove(0);
            }
        }
    }
    
    expect_char(tokens, "(")?;
    let to_var = expect_identifier(tokens)?;
    let to_label = if peek_token(tokens) == ":" {
        tokens.remove(0);
        Some(expect_identifier(tokens)?)
    } else {
        None
    };
    expect_char(tokens, ")")?;
    
    Ok(MatchPattern::Relationship {
        from: NodePattern {
            variable: from_var,
            label: from_label,
        },
        edge: EdgePattern {
            direction,
            label: edge_label,
        },
        to: NodePattern {
            variable: to_var,
            label: to_label,
        },
    })
}

fn parse_where(tokens: &mut Vec<String>) -> Result<Option<WhereClause>, ParseError> {
    if tokens.is_empty() || tokens[0].to_uppercase() != "WHERE" {
        return Ok(None);
    }
    
    tokens.remove(0);
    
    let variable = expect_identifier(tokens)?;
    expect_char(tokens, ".")?;
    let field = expect_identifier(tokens)?;
    expect_char(tokens, "=")?;
    
    if field == "id" {
        let num = expect_number(tokens)?;
        Ok(Some(WhereClause::NodeIdEq {
            variable,
            value: num as u128,
        }))
    } else {
        let str_value = expect_string(tokens)?;
        Ok(Some(WhereClause::NodeAttrEq {
            variable,
            attr: field,
            value: str_value,
        }))
    }
}

fn parse_return(tokens: &mut Vec<String>) -> Result<ReturnClause, ParseError> {
    expect_keyword(tokens, "RETURN")?;
    
    if peek_token(tokens).to_uppercase() == "*" {
        tokens.remove(0);
        return Ok(ReturnClause::All);
    }
    
    let variable = expect_identifier(tokens)?;
    
    if peek_token(tokens) == "." {
        tokens.remove(0);
        let attr = expect_identifier(tokens)?;
        Ok(ReturnClause::NodeAttr { variable, attr })
    } else {
        Ok(ReturnClause::NodeId { variable })
    }
}

fn parse_limit(tokens: &mut Vec<String>) -> Result<Option<usize>, ParseError> {
    if tokens.is_empty() || tokens[0].to_uppercase() != "LIMIT" {
        return Ok(None);
    }
    
    tokens.remove(0);
    let limit = expect_number(tokens)?;
    Ok(Some(limit))
}

fn expect_keyword(tokens: &mut Vec<String>, keyword: &str) -> Result<(), ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::UnexpectedToken(format!("Expected '{}'", keyword)));
    }
    
    if tokens[0].to_uppercase() != keyword.to_uppercase() {
        return Err(ParseError::UnexpectedToken(format!("Expected '{}', got '{}'", keyword, tokens[0])));
    }
    
    tokens.remove(0);
    Ok(())
}

fn expect_char(tokens: &mut Vec<String>, ch: &str) -> Result<(), ParseError> {
    if tokens.is_empty() || tokens[0] != ch {
        return Err(ParseError::UnexpectedToken(format!("Expected '{}'", ch)));
    }
    
    tokens.remove(0);
    Ok(())
}

fn expect_identifier(tokens: &mut Vec<String>) -> Result<String, ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::UnexpectedToken("Expected identifier".to_string()));
    }
    
    let token = tokens.remove(0);
    if token.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
        Ok(token)
    } else {
        Err(ParseError::UnexpectedToken(format!("Expected identifier, got '{}'", token)))
    }
}

fn expect_number(tokens: &mut Vec<String>) -> Result<usize, ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::UnexpectedToken("Expected number".to_string()));
    }
    
    let token = tokens.remove(0);
    token.parse::<usize>()
        .map_err(|_| ParseError::InvalidSyntax(format!("Expected number, got '{}'", token)))
}

fn expect_string(tokens: &mut Vec<String>) -> Result<String, ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::UnexpectedToken("Expected string".to_string()));
    }
    
    let token = tokens.remove(0);
    Ok(token.trim_matches('\'').trim_matches('"').to_string())
}

fn peek_token(tokens: &[String]) -> &str {
    if tokens.is_empty() {
        ""
    } else {
        &tokens[0]
    }
}

pub fn compile_to_opcodes(query: CypherQuery) -> Vec<Opcode> {
    let mut opcodes = Vec::new();
    
    match query.match_pattern {
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
            if let Some(start_id) = extract_start_node_id(&query.where_clause) {
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
    
    if let Some(limit) = query.limit {
        opcodes.push(Opcode::SetLimit(limit));
    }
    
    opcodes.push(Opcode::SaveResults);
    
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

    #[test]
    fn test_parse_simple_match() {
        let query = "MATCH (n:User) RETURN n.id LIMIT 10";
        let result = parse(query);
        assert!(result.is_ok());
        
        let query = result.unwrap();
        match query.match_pattern {
            MatchPattern::SingleNode { variable, label } => {
                assert_eq!(variable, "n");
                assert_eq!(label, Some("User".to_string()));
            }
            _ => panic!("Expected SingleNode pattern"),
        }
    }


    #[test]
    fn test_parse_single_node_without_label() {
        let query = "MATCH (n) RETURN n.id LIMIT 10";
        let result = parse(query);
        assert!(result.is_ok());
        
        let query = result.unwrap();
        match query.match_pattern {
            MatchPattern::SingleNode { variable, label } => {
                assert_eq!(variable, "n");
                assert_eq!(label, None);
            }
            _ => panic!("Expected SingleNode pattern"),
        }
    }


    #[test]
    fn test_parse_return_all() {
        let query = "MATCH (n:User) RETURN * LIMIT 10";
        let result = parse(query);
        assert!(result.is_ok());
        
        let query = result.unwrap();
        match query.return_clause {
            ReturnClause::All => {}
            _ => panic!("Expected All return clause"),
        }
    }

    #[test]
    fn test_parse_missing_limit() {
        let query = "MATCH (n:User) RETURN n.id";
        let result = parse(query);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            ParseError::MissingLimit => {}
            _ => panic!("Expected MissingLimit error"),
        }
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let query = "MATCH (n:User RETURN n.id LIMIT 10";
        let result = parse(query);
        assert!(result.is_err());
    }


    #[test]
    fn test_compile_relationship_query() {
        let query = CypherQuery {
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
    fn test_tokenize_basic() {
        let result = tokenize("MATCH (n:User) RETURN n.id LIMIT 10");
        assert!(result.is_ok());
        
        let tokens = result.unwrap();
        assert!(tokens.contains(&"MATCH".to_string()));
        assert!(tokens.contains(&"(".to_string()));
        assert!(tokens.contains(&"n".to_string()));
    }

    #[test]
    fn test_tokenize_with_strings() {
        let result = tokenize("WHERE n.name = 'John'");
        assert!(result.is_ok());
        
        let tokens = result.unwrap();
        assert!(tokens.contains(&"John".to_string()));
    }


    #[test]
    fn test_parse_multiple_whitespace() {
        let query = "MATCH   (n:User)   RETURN   n.id   LIMIT   10";
        let result = parse(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_case_insensitive_keywords() {
        let query = "match (n:User) return n.id limit 10";
        let result = parse(query);
        assert!(result.is_ok());
    }


    #[test]
    fn test_compile_with_start_node_id() {
        let query = CypherQuery {
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

    #[test]
    fn test_parse_empty_query() {
        let query = "";
        let result = parse(query);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_match() {
        let query = "RETURN n.id LIMIT 10";
        let result = parse(query);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_return() {
        let query = "MATCH (n:User) LIMIT 10";
        let result = parse(query);
        assert!(result.is_err());
    }
}

