# Sol Micro SQL

Graph database with Cypher-like queries for Solana.

## What it does

Store graphs on-chain and query them using a Cypher-inspired language. Queries are compiled to bytecode and executed on a custom VM.

## Query Example

```cypher
MATCH (n:User)-[:FOLLOWS]->(m:User)
WHERE n.id = 42
RETURN m.id LIMIT 10
```

## How it works

1. Parse Cypher query → AST
2. Compile AST → VM opcodes
3. Execute opcodes on graph → results

## Features

- Store nodes and edges on-chain
- Traverse graph with filters (node/edge labels, positive/negative)
- Cypher-like query syntax
- Custom VM for query execution

## Status

✅ Graph storage and traversal  
✅ Cypher parser  
✅ Code generation  
✅ VM execution  

🚧 Extended patterns and filters

## Build & Test

```bash
anchor build
anchor test
```
