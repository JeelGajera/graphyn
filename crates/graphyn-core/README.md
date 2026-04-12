# graphyn-core

Language-agnostic graph engine for Graphyn.

`graphyn-core` owns the canonical IR, graph structure, alias resolution, and query algorithms. It does not parse source code directly and does not contain language-specific logic.

## Responsibilities

- Defines the frozen IR contract (`Symbol`, `Relationship`, `FileIR`, `RepoIR`)
- Stores a directed symbol graph (`GraphynGraph`)
- Resolves alias chains (import aliases, re-exports, barrel/default alias metadata)
- Provides query APIs:
  - `blast_radius`
  - `dependencies`
  - `symbol_usages`
- Supports incremental graph update plumbing

## Public modules

- `ir`: shared IR contract used by all adapters
- `graph`: graph container + indexes
- `resolver`: alias chain ingestion and canonicalization helpers
- `query`: traversal-based query functions
- `incremental`: file-level replacement/update helpers
- `error`: `GraphynError`

## Minimal usage

```rust
use graphyn_core::graph::GraphynGraph;
use graphyn_core::query;

let graph = GraphynGraph::new();
let _ = query::blast_radius(&graph, "UserPayload", None, Some(2));
```

## Notes

- This crate is deterministic by design and contains no LLM logic.
- Language parsing belongs in adapter crates (for example, `graphyn-adapter-ts`).
