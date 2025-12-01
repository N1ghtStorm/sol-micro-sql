#[derive(Debug, Clone)]
pub enum CypherQuery {
    Match {
        match_pattern: MatchPattern,
        where_clause: Option<WhereClause>,
        return_clause: ReturnClause,
        limit: Option<usize>,
    },
    Create {
        create_pattern: CreatePattern,
    },
}

#[derive(Debug, Clone)]
pub enum CreatePattern {
    Node {
        variable: String,
        label: Option<String>,
        data: Option<Vec<u8>>, // Node data in hex format
    },
    Edge {
        from: NodePattern,
        from_id: Option<u128>, // Node ID if specified directly
        edge: EdgePattern,
        to: NodePattern,
        to_id: Option<u128>, // Node ID if specified directly
    },
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
    NodeId { variable: String },
    NodeAttr { variable: String, attr: String },
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

    if tokens.is_empty() {
        return Err(ParseError::InvalidSyntax("Empty query".to_string()));
    }

    let first_token = tokens[0].to_uppercase();
    if first_token == "CREATE" {
        let create_pattern = parse_create(&mut tokens)?;
        if !tokens.is_empty() {
            return Err(ParseError::InvalidSyntax(format!(
                "Unexpected tokens: {:?}",
                tokens
            )));
        }
        Ok(CypherQuery::Create { create_pattern })
    } else if first_token == "MATCH" {
        let match_pattern = parse_match(&mut tokens)?;
        let where_clause = parse_where(&mut tokens)?;
        let return_clause = parse_return(&mut tokens)?;
        let limit = parse_limit(&mut tokens)?;

        if limit.is_none() {
            return Err(ParseError::MissingLimit);
        }

        if !tokens.is_empty() {
            return Err(ParseError::InvalidSyntax(format!(
                "Unexpected tokens: {:?}",
                tokens
            )));
        }

        Ok(CypherQuery::Match {
            match_pattern,
            where_clause,
            return_clause,
            limit,
        })
    } else {
        Err(ParseError::InvalidSyntax(format!(
            "Expected MATCH or CREATE, got '{}'",
            tokens[0]
        )))
    }
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
            '(' | ')' | '[' | ']' | '-' | '>' | '<' | ':' | '=' | ',' | '{' | '}' => {
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

fn parse_create(tokens: &mut Vec<String>) -> Result<CreatePattern, ParseError> {
    expect_keyword(tokens, "CREATE")?;

    if tokens.is_empty() {
        return Err(ParseError::InvalidSyntax(
            "Expected pattern after CREATE".to_string(),
        ));
    }

    let has_arrow = tokens.iter().any(|t| t == "->" || t == "<-" || t == "-");
    if has_arrow {
        parse_create_edge_pattern(tokens)
    } else {
        parse_create_node_pattern(tokens)
    }
}

fn parse_create_node_pattern(tokens: &mut Vec<String>) -> Result<CreatePattern, ParseError> {
    expect_char(tokens, "(")?;

    let variable = expect_identifier(tokens)?;
    let label = if peek_token(tokens) == ":" {
        tokens.remove(0);
        Some(expect_identifier(tokens)?)
    } else {
        None
    };

    // Parse data in format { 0x.... }
    let data = if peek_token(tokens) == "{" {
        tokens.remove(0);
        // Expect hex string starting with 0x
        if peek_token(tokens).starts_with("0x") || peek_token(tokens).starts_with("0X") {
            let hex_str = tokens.remove(0);
            // Remove 0x prefix and parse hex
            let hex_bytes = hex_str.trim_start_matches("0x").trim_start_matches("0X");
            let parsed_data = parse_hex_string(hex_bytes)
                .map_err(|e| ParseError::InvalidSyntax(format!("Invalid hex string: {}", e)))?;
            expect_char(tokens, "}")?;
            Some(parsed_data)
        } else {
            return Err(ParseError::InvalidSyntax(
                "Expected hex string starting with 0x".to_string(),
            ));
        }
    } else {
        None
    };

    expect_char(tokens, ")")?;

    Ok(CreatePattern::Node {
        variable,
        label,
        data,
    })
}

fn parse_create_edge_pattern(tokens: &mut Vec<String>) -> Result<CreatePattern, ParseError> {
    expect_char(tokens, "(")?;

    // Support both identifier (variable) and numeric ID
    let from_token = if tokens.is_empty() {
        return Err(ParseError::UnexpectedToken(
            "Expected node identifier or ID".to_string(),
        ));
    } else {
        tokens.remove(0)
    };

    let (from_var, from_id, from_label) = if from_token
        .chars()
        .next()
        .map(|c| c.is_alphabetic() || c == '_')
        .unwrap_or(false)
    {
        // It's a variable identifier
        let label = if peek_token(tokens) == ":" {
            tokens.remove(0);
            Some(expect_identifier(tokens)?)
        } else {
            None
        };
        expect_char(tokens, ")")?;
        (Some(from_token), None, label)
    } else if from_token.chars().all(|c| c.is_ascii_digit()) {
        // It's a numeric ID
        let from_id = from_token
            .parse::<u128>()
            .map_err(|_| ParseError::InvalidSyntax(format!("Invalid node ID: {}", from_token)))?;
        expect_char(tokens, ")")?;
        (None, Some(from_id), None)
    } else {
        return Err(ParseError::InvalidSyntax(format!(
            "Expected node identifier or ID, got '{}'",
            from_token
        )));
    };

    // Parse edge pattern: -[:LABEL]-> or <-[:LABEL]- or -[:LABEL]-
    expect_char(tokens, "-")?;

    // Check if next is [ (edge label) or >/< (direction)
    let direction = if peek_token(tokens) == "[" {
        // Edge label comes first, direction will be determined after ]
        EdgeDirection::Bidirectional // Temporary, will be updated after parsing label
    } else if peek_token(tokens) == ">" {
        tokens.remove(0);
        EdgeDirection::Outgoing
    } else if peek_token(tokens) == "<" {
        tokens.remove(0);
        EdgeDirection::Incoming
    } else {
        EdgeDirection::Bidirectional
    };

    // Parse edge label if present
    let edge_label = if peek_token(tokens) == "[" {
        tokens.remove(0);
        let label = if peek_token(tokens) == ":" {
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
        label
    } else {
        None
    };

    // Determine final direction based on what comes after the label
    let final_direction = if peek_token(tokens) == "-" {
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
    } else if peek_token(tokens) == ">" {
        tokens.remove(0);
        EdgeDirection::Outgoing
    } else if peek_token(tokens) == "<" {
        tokens.remove(0);
        EdgeDirection::Incoming
    } else {
        direction // Use the direction we determined earlier
    };

    expect_char(tokens, "(")?;

    // Support both identifier (variable) and numeric ID for 'to' node
    let to_token = if tokens.is_empty() {
        return Err(ParseError::UnexpectedToken(
            "Expected node identifier or ID".to_string(),
        ));
    } else {
        tokens.remove(0)
    };

    let (to_var, to_id, to_label) = if to_token
        .chars()
        .next()
        .map(|c| c.is_alphabetic() || c == '_')
        .unwrap_or(false)
    {
        // It's a variable identifier
        let label = if peek_token(tokens) == ":" {
            tokens.remove(0);
            Some(expect_identifier(tokens)?)
        } else {
            None
        };
        expect_char(tokens, ")")?;
        (Some(to_token), None, label)
    } else if to_token.chars().all(|c| c.is_ascii_digit()) {
        // It's a numeric ID
        let to_id = to_token
            .parse::<u128>()
            .map_err(|_| ParseError::InvalidSyntax(format!("Invalid node ID: {}", to_token)))?;
        expect_char(tokens, ")")?;
        (None, Some(to_id), None)
    } else {
        return Err(ParseError::InvalidSyntax(format!(
            "Expected node identifier or ID, got '{}'",
            to_token
        )));
    };

    // Store node IDs in the pattern for CREATE edge
    Ok(CreatePattern::Edge {
        from: NodePattern {
            variable: from_var.unwrap_or_default(),
            label: from_label,
        },
        from_id: from_id,
        edge: EdgePattern {
            direction: final_direction,
            label: edge_label,
        },
        to: NodePattern {
            variable: to_var.unwrap_or_default(),
            label: to_label,
        },
        to_id: to_id,
    })
}

fn parse_match(tokens: &mut Vec<String>) -> Result<MatchPattern, ParseError> {
    expect_keyword(tokens, "MATCH")?;

    if tokens.is_empty() {
        return Err(ParseError::InvalidSyntax(
            "Expected pattern after MATCH".to_string(),
        ));
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
        return Err(ParseError::InvalidSyntax(
            "Expected edge pattern".to_string(),
        ));
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
        return Err(ParseError::UnexpectedToken(format!(
            "Expected '{}'",
            keyword
        )));
    }

    if tokens[0].to_uppercase() != keyword.to_uppercase() {
        return Err(ParseError::UnexpectedToken(format!(
            "Expected '{}', got '{}'",
            keyword, tokens[0]
        )));
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
        return Err(ParseError::UnexpectedToken(
            "Expected identifier".to_string(),
        ));
    }

    let token = tokens.remove(0);
    if token
        .chars()
        .next()
        .map(|c| c.is_alphabetic() || c == '_')
        .unwrap_or(false)
    {
        Ok(token)
    } else {
        Err(ParseError::UnexpectedToken(format!(
            "Expected identifier, got '{}'",
            token
        )))
    }
}

fn expect_number(tokens: &mut Vec<String>) -> Result<usize, ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::UnexpectedToken("Expected number".to_string()));
    }

    let token = tokens.remove(0);
    token
        .parse::<usize>()
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

fn parse_hex_string(hex: &str) -> Result<Vec<u8>, String> {
    // Remove any whitespace
    let hex = hex.trim();

    // Hex string must have even number of characters
    if hex.len() % 2 != 0 {
        return Err("Hex string must have even number of characters".to_string());
    }

    let mut bytes = Vec::new();
    for i in (0..hex.len()).step_by(2) {
        let byte_str = &hex[i..i + 2];
        let byte = u8::from_str_radix(byte_str, 16)
            .map_err(|e| format!("Invalid hex character: {}", e))?;
        bytes.push(byte);
    }

    Ok(bytes)
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
        match query {
            CypherQuery::Match { match_pattern, .. } => match match_pattern {
                MatchPattern::SingleNode { variable, label } => {
                    assert_eq!(variable, "n");
                    assert_eq!(label, Some("User".to_string()));
                }
                _ => panic!("Expected SingleNode pattern"),
            },
            _ => panic!("Expected Match query"),
        }
    }

    #[test]
    fn test_parse_single_node_without_label() {
        let query = "MATCH (n) RETURN n.id LIMIT 10";
        let result = parse(query);
        assert!(result.is_ok());

        let query = result.unwrap();
        match query {
            CypherQuery::Match { match_pattern, .. } => match match_pattern {
                MatchPattern::SingleNode { variable, label } => {
                    assert_eq!(variable, "n");
                    assert_eq!(label, None);
                }
                _ => panic!("Expected SingleNode pattern"),
            },
            _ => panic!("Expected Match query"),
        }
    }

    #[test]
    fn test_parse_return_all() {
        let query = "MATCH (n:User) RETURN * LIMIT 10";
        let result = parse(query);
        assert!(result.is_ok());

        let query = result.unwrap();
        match query {
            CypherQuery::Match { return_clause, .. } => match return_clause {
                ReturnClause::All => {}
                _ => panic!("Expected All return clause"),
            },
            _ => panic!("Expected Match query"),
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

    #[test]
    fn test_parse_create_node() {
        let query = "CREATE (n:Person)";
        let result = parse(query);
        assert!(result.is_ok());

        let query = result.unwrap();
        match query {
            CypherQuery::Create { create_pattern } => match create_pattern {
                CreatePattern::Node {
                    variable,
                    label,
                    data,
                } => {
                    assert_eq!(variable, "n");
                    assert_eq!(label, Some("Person".to_string()));
                    assert_eq!(data, None);
                }
                _ => panic!("Expected Node create pattern"),
            },
            _ => panic!("Expected Create query"),
        }
    }

    #[test]
    fn test_parse_create_node_with_hex_data() {
        let query = "CREATE (n:Person {0x1234})";
        let result = parse(query);
        assert!(result.is_ok());

        let query = result.unwrap();
        match query {
            CypherQuery::Create { create_pattern } => match create_pattern {
                CreatePattern::Node {
                    variable,
                    label,
                    data,
                } => {
                    assert_eq!(variable, "n");
                    assert_eq!(label, Some("Person".to_string()));
                    assert_eq!(data, Some(vec![0x12, 0x34]));
                }
                _ => panic!("Expected Node create pattern"),
            },
            _ => panic!("Expected Create query"),
        }
    }

    #[test]
    fn test_parse_create_edge_with_ids() {
        let query = "CREATE (1)-[:FOLLOWS]->(2)";
        let result = parse(query);
        assert!(result.is_ok());

        let query = result.unwrap();
        match query {
            CypherQuery::Create { create_pattern } => match create_pattern {
                CreatePattern::Edge {
                    from_id,
                    to_id,
                    edge,
                    ..
                } => {
                    assert_eq!(from_id, Some(1));
                    assert_eq!(to_id, Some(2));
                    assert_eq!(edge.label, Some("FOLLOWS".to_string()));
                }
                _ => panic!("Expected Edge create pattern"),
            },
            _ => panic!("Expected Create query"),
        }
    }

    #[test]
    fn test_parse_create_edge_with_variables() {
        let query = "CREATE (a:User)-[:KNOWS]->(b:User)";
        let result = parse(query);
        assert!(result.is_ok());

        let query = result.unwrap();
        match query {
            CypherQuery::Create { create_pattern } => {
                match create_pattern {
                    CreatePattern::Edge {
                        from_id,
                        to_id,
                        edge,
                        from,
                        to,
                    } => {
                        // Variables are used, so IDs should be None
                        assert_eq!(from_id, None);
                        assert_eq!(to_id, None);
                        assert_eq!(from.variable, "a");
                        assert_eq!(to.variable, "b");
                        assert_eq!(edge.label, Some("KNOWS".to_string()));
                    }
                    _ => panic!("Expected Edge create pattern"),
                }
            }
            _ => panic!("Expected Create query"),
        }
    }
}
