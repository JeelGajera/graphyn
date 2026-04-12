# Graphyn Architecture

This document describes the internal architecture of Graphyn — how the crates relate, how data flows from source code to query results, and the reasoning behind each design decision.

---

## Overview

Graphyn is a Rust workspace. Each crate has exactly one responsibility. No crate reaches outside its defined scope.

```
graphyn-core          Language-agnostic graph engine. No language knowledge.
graphyn-adapter-ts    TypeScript/JavaScript → IR. No graph knowledge.
graphyn-store         Graph persistence and cache. No parsing or query logic.
graphyn-mcp           MCP protocol server. Calls into core. No parsing.
graphyn-cli           Developer CLI. Orchestrates everything. No business logic.
```

Dependency direction is strictly one-way:

```
graphyn-cli
  ├── graphyn-mcp → graphyn-core
  ├── graphyn-adapter-ts → graphyn-core
  └── graphyn-store → graphyn-core
```

`graphyn-core` has no dependencies on any other Graphyn crate. It is the foundation.

---

## Data flow

```
Source files on disk
        ↓
   graphyn-adapter-ts
   (tree-sitter parse → FileIR per file, parallel via rayon)
        ↓
   RepoIR
   (Vec<FileIR> + language stats)
        ↓
   graphyn-core: graph builder
   (Symbol nodes inserted, Relationship edges inserted, alias chains built)
        ↓
   GraphynGraph
   (petgraph DiGraph + DashMap indexes + AliasChains)
        ↓
   graphyn-store
   (serialize to RocksDB → .graphyn/db)

   On subsequent startup:
   graphyn-store deserialize → GraphynGraph (< 2s, no reparse)

   On query:
   graphyn-core query.rs
   (BFS/DFS traversal of in-memory graph, < 100ms)
        ↓
   graphyn-mcp context_builder.rs
   (format result for agent consumption)
        ↓
   MCP tool response (JSON via stdio)
```

---

## graphyn-core

The heart of the system. Language-agnostic. Receives IR, builds graph, answers queries.

### ir.rs

The IR schema. `Symbol`, `Relationship`, `FileIR`, `RepoIR`. These are the types that every language adapter must produce and that every query returns results against. Frozen for v1 — see [ir-spec.md](ir-spec.md).

### graph.rs

The `GraphynGraph` struct. Contains:

```rust
pub struct GraphynGraph {
    // The actual directed graph. Node payload = SymbolId. Edge payload = RelationshipMeta.
    pub graph: DiGraph<SymbolId, RelationshipMeta>,

    // Fast lookup: SymbolId → NodeIndex in petgraph
    pub node_index: DashMap<SymbolId, NodeIndex>,

    // Fast lookup: symbol name → Vec<SymbolId>
    // Multiple symbols can share a name (same name in different files)
    pub name_index: DashMap<String, Vec<SymbolId>>,

    // Fast lookup: file path → Vec<SymbolId>
    pub file_index: DashMap<String, Vec<SymbolId>>,

    // Full symbol data
    pub symbols: DashMap<SymbolId, Symbol>,

    // Alias chains: canonical SymbolId → all known aliases
    pub alias_chains: DashMap<SymbolId, Vec<AliasEntry>>,
}
```

`DashMap` is used instead of `HashMap` throughout because graph mutations (incremental updates) may happen concurrently with read queries. `DashMap` provides lock-free concurrent access without requiring `RwLock` on the outer struct.

`petgraph` provides the directed graph structure and the BFS/DFS traversal algorithms used by the query engine.

### resolver.rs

Alias resolution. This is the most important module in the system.

When the graph is built from IR, every `Relationship` with a non-None `alias` field is registered in `alias_chains`:

```
UserPayload::class → [
    AliasEntry { alias_name: "ResponseModel", defined_in: "mappers/view_model_mapper.ts", scope: ImportAlias },
    AliasEntry { alias_name: "PublicUser", defined_in: "index.ts", scope: ReExport },
]
```

When `get_blast_radius("UserPayload")` is called, the resolver expands the query to include all known aliases before traversing the graph. This is what catches the aliased import bug.

Alias scopes:
- `ImportAlias` — `import { A as B }`
- `ReExport` — `export { A as B } from './a'`
- `BarrelReExport` — `export * from './a'` (exposes original names)
- `DefaultImport` — `import B from './a'` where B was the default export of A

### query.rs

Three query functions that traverse the graph:

**`blast_radius(symbol, depth)`**
BFS from target node, following INCOMING edges (who points at this?).
At each hop, include alias metadata from edge payload.
Returns `BlastRadiusResult` with `direct` and `aliased` groups.

**`dependencies(symbol, depth)`**
BFS from target node, following OUTGOING edges (what does this point at?).
Returns `DependencyResult`.

**`symbol_usages(symbol)`**
Looks up canonical symbol + all alias names in `name_index`.
Collects all incoming edges across all alias names.
Deduplicates by `file + line`.
Returns `UsageResult`.

### index.rs

Fast reverse lookup tables. `name_index` and `file_index` exist here as secondary indexes over the graph. They are rebuilt from the graph on load and updated incrementally on file change.

### incremental.rs

Handles partial graph updates when a file changes.

On a file-change event from the watcher:
1. Remove all nodes and edges from the changed file (using `file_index`)
2. Re-parse the changed file using the appropriate adapter
3. Insert new nodes and edges from the fresh `FileIR`
4. Rebuild alias chains for affected symbols
5. Update `name_index` and `file_index`

The rest of the graph is untouched. This is what keeps incremental update time under 500ms.

---

## graphyn-adapter-ts

TypeScript and JavaScript parser. Uses tree-sitter for parsing — no dependency on the TypeScript compiler, no `node_modules` required.

### walker.rs

Discovers all `.ts`, `.tsx`, `.js`, `.jsx` files under the repo root. Respects `.gitignore` patterns. Skips `node_modules`, `dist`, `build`, `.graphyn` directories automatically.

### parser.rs

Calls `tree-sitter-typescript` or `tree-sitter-javascript` on each file. Returns a tree-sitter `Tree`. Non-fatal on parse errors — logs them and returns a partial tree.

### extractor.rs

Walks the tree-sitter AST and extracts `Symbol` and `Relationship` instances. This is the largest module. Key responsibilities:

- Detect class declarations, function declarations, interface declarations, type aliases, enum declarations
- Detect import statements (named, default, namespace, aliased, dynamic)
- Detect property accesses and function calls
- Assign stable `SymbolId` values to every found symbol
- Populate `properties_accessed` on relationships where property accesses are present

### import_resolver.rs

Handles the complex import patterns:

- `import { A as B }` — aliased named import
- `import * as NS from './module'` — namespace import
- `export { A } from './b'` — re-export
- `export * from './b'` — barrel re-export
- `export default class A` — default export
- `import A from './b'` — default import

Barrel file handling: when `index.ts` contains `export * from './user_payload'`, the resolver must follow the re-export chain and register all re-exported symbols with the barrel's path so they can be found when imported from the barrel.

---

## graphyn-store

Persistence and caching.

### db.rs

Serializes the full `GraphynGraph` to RocksDB at `.graphyn/db` in the repo root. Uses `serde_json` for serialization. On startup, deserializes and validates freshness by comparing file modification times against the stored index — files modified since the last store trigger a targeted re-parse rather than a full rebuild.

### cache.rs

LRU cache for frequently-queried symbols. Blast radius results for hot symbols (e.g. a base class used everywhere) are cached after first computation. Cache is invalidated for a symbol when any of its dependents change.

---

## graphyn-mcp

MCP server. Receives tool calls from agents via stdio, delegates to `graphyn-core`, formats results.

### server.rs

Uses the `rmcp` crate for MCP protocol handling. Registers three tools on startup: `get_blast_radius`, `get_dependencies`, `get_symbol_usages`. Each tool has a JSON schema generated via `schemars` so agents receive properly typed tool definitions.

### context_builder.rs

Formats query results from `graphyn-core` into agent-friendly text. The raw graph result is a Rust struct — context_builder turns it into the structured text format shown in the README. Key design goal: the output should be immediately actionable by an agent with no additional reasoning required.

### tools/

One file per MCP tool. Each file:
1. Receives the tool call parameters (JSON)
2. Validates and deserializes parameters
3. Calls the appropriate `query.rs` function
4. Passes result to `context_builder.rs`
5. Returns the formatted string as the MCP tool response

---

## graphyn-cli

Developer-facing CLI. Orchestrates the other crates. Contains no business logic.

### commands/analyze.rs

1. Discovers all source files (adapter walker)
2. Parses them in parallel (rayon + adapter parser + extractor)
3. Builds the graph (graphyn-core)
4. Persists to RocksDB (graphyn-store)
5. Prints stats: files parsed, symbols found, relationships found, time taken

### commands/query.rs

1. Loads graph from RocksDB (graphyn-store)
2. Calls the appropriate query function (graphyn-core)
3. Formats output as a terminal table

### commands/watch.rs

1. Loads graph from RocksDB
2. Starts the MCP server
3. Starts the file watcher (notify crate)
4. On file change: incremental update (graphyn-core) + persist delta + notify MCP server

### commands/serve.rs

Starts the MCP server in stdio mode. Designed to be called by agent MCP configurations directly.

---

## Performance design

### Why in-memory

Queries must return in under 100ms. Any disk or network I/O on the query path makes this impossible at scale. The graph lives entirely in RAM after startup. `petgraph` + `DashMap` together use approximately 1–2 bytes per node and 2–4 bytes per edge for the index structures. A 500k LOC TypeScript codebase produces roughly 50k–100k symbols and 200k–400k relationships — this fits comfortably in under 500MB.

### Why rayon for parsing

Parsing is embarrassingly parallel — each file is independent. `rayon` distributes file parsing across all available CPU cores automatically. A 300-API codebase with 50k LOC parses in approximately 8 seconds on a single machine because 8 cores parse 8 files simultaneously.

### Why DashMap

Graph mutations (incremental updates) and read queries must be able to happen concurrently. `DashMap` is a lock-free concurrent hash map — it shards internally so reads and writes to different symbols do not block each other. This is critical for watch mode where a file change triggers a mutation while the MCP server is simultaneously serving queries.

### Why RocksDB

RocksDB provides fast key-value storage with good compression. A serialized 100k-symbol graph occupies approximately 20–30MB on disk. Deserialization (startup after the first run) takes under 2 seconds because RocksDB's read path is heavily optimized for sequential key reads.

---

## Adding a new language adapter

To add Python support (v2), the process is:

1. Create `crates/graphyn-adapter-python/`
2. Add `tree-sitter-python` as a dependency
3. Implement `walker.rs` — find `.py` files, skip `__pycache__`, `.venv`, etc.
4. Implement `parser.rs` — tree-sitter parse per file
5. Implement `extractor.rs` — extract symbols and relationships into IR
6. Implement `import_resolver.rs` — handle Python's import patterns:
   - `from module import Class as Alias`
   - `import module` then `module.Class` usage
   - `from . import module` (relative imports)
   - `__init__.py` barrel equivalents
7. Add `fixtures/python-sample/` with representative Python code
8. Write tests — the alias-import equivalent for Python must pass
9. Wire into `graphyn-cli analyze` as a new adapter for `.py` files

`graphyn-core` is not touched. The IR schema is not changed. The MCP tools are not changed.