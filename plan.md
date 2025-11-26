# Graph Query VM — High-Level Design (Solana / Rust)

A short conceptual plan for a **graph database + query language + VM + post-quantum authorization** built as a Solana smart contract.

---

## 1. Goal

- Store a graph on-chain.  
- Define a small Cypher-lite query language.  
- Compile queries into bytecode.  
- Execute bytecode inside a custom VM.  
- Authorize every query with post-quantum (PQ) signatures or a PQ commit-reveal scheme.

---

## 2. Graph Storage (MVP)

Single account `GraphStore` containing:

- `node_count`, `edge_count`  
- Array of `Node`  
- Array of `Edge`

**Node:** `id`, `label`, simple attributes, index of outgoing edges.  
**Edge:** `from`, `to`, `label`.

Later: pagination, multi-account graph sharding.

---

## 3. Query Language (Cypher-lite)

Example:


```
MATCH (n:User)-[:FOLLOWS]->(m:User)
WHERE n.id = 42
RETURN m.id LIMIT 10
```


MVP rules:

- Pattern length 1–2 hops.  
- Simple filters only (`id`, basic attrs).  
- Must include LIMIT.  
- No recursion, no loops.

---

## 4. VM Design (Bytecode Interpreter)

State:

- Value stack  
- `current_set` of node_ids  
- `result_set`  
- `step_limit` to avoid CU explosion

Basic instructions:

- `PUSH_CONST_U64`  
- `SET_CURRENT_FROM_ALL_NODES`  
- `FILTER_NODE_LABEL`  
- `FILTER_NODE_ATTR_EQ`  
- `TRAVERSE_OUT <label>`  
- `LIMIT <k>`  
- `SAVE_RESULTS`  
- `HALT`

DSL → AST → bytecode (compiled off-chain).

---

## 5. PQ Authorization Layer

Goal: only PQ-approved queries can run.

### Option A — “Mock PQ”
Off-chain:  
- Generate PQ keys, sign `code_hash`.

On-chain:  
- `QueryAuth` account stores `pq_pubkey`, `code_hash`, `pq_signature` (blob).  
- Contract checks `code_hash` matches bytecode and signature commitments.

### Option B — Commit-Reveal + PQ
- Commit: `H(pq_pubkey || code_hash)` stored on-chain.  
- Reveal: send PQ data; contract checks commit match + time window.

Contract **never executes** bytecode without valid PQ auth.

---

## 6. Query Execution Flow

**Off-chain:**  
1. Write DSL query → compile to bytecode.  
2. Compute `code_hash`.  
3. Produce PQ signature or commit.  
4. Send transaction with:
   - GraphStore  
   - Bytecode account  
   - QueryAuth  
   - Params (start node, etc.)

**On-chain:**  
1. Validate PQ layer.  
2. Initialize VM.  
3. Run bytecode until `HALT` or `step_limit`.  
4. Write result set to `ResultAccount` or logs.

---

## 7. Roadmap

1. **Graph-only MVP:** node/edge ops.  
2. **VM MVP:** bytecode interpreter without PQ.  
3. **DSL Compiler:** text → AST → bytecode.  
4. **Add PQ Authorization:** QueryAuth + commit-reveal.  
5. **Extended Queries:** multi-hop, more patterns, graph sharding.

