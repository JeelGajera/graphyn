# Graphyn

> Understand the blast radius before you pull the trigger.

Graphyn is a code intelligence engine that models your codebase as a living graph of symbol relationships. It gives coding agents a precise knowledge of what will break before making a change or how a change will affect the codebase.

It is not a search tool. It is not a chatbot over your repo. It is a deterministic relationship graph that resolves aliases, tracks property-level access, and answers the questions your agent needs answered before touching anything.

---

## The problem

You change a class. Your coding agent searches for usages, reads the files it finds, and makes changes. Three days later something breaks in production — a mapper three directories deep imported that class under a different name. The agent never found it because it was looking for the original name.

This is not a search problem. It is a relationship graph problem.

```typescript
// src/models/user_payload.ts
export class UserPayload {
  userId: string;
  timestamp: Date;
  status: string;
}

// src/mappers/response/deep/view_model_mapper.ts
import { UserPayload as ResponseModel } from '../../../models/user_payload';
//                    ^^^^^^^^^^^^^^^ different name — agent missed this

export class ViewModelMapper {
  map(data: ResponseModel): object {
    return {
      id: data.userId,      // silently broken after your change
      ts: data.timestamp,
      st: data.status,
    };
  }
}
```

Graphyn catches this. Always.

---

## What Graphyn tells you

Given any symbol — class, function, type, interface — Graphyn answers:

**Blast radius** — what will break if you change this:
```
Symbol: UserPayload [class] — src/models/user_payload.ts:12

Blast radius (3 dependents):

DIRECT:
  • src/handlers/auth.ts:45
    → imports as UserPayload
    → accesses: .userId, .email

  • src/handlers/profile.ts:23
    → imports as UserPayload
    → accesses: .userId, .timestamp

ALIASED (high risk — different import name):
  • src/mappers/response/deep/view_model_mapper.ts:8
    → imports as ResponseModel  ← ALIAS
    → accesses: .userId, .timestamp, .status

Properties at risk: .userId (3 files), .timestamp (2 files), .status (1 file)
```

**Dependencies** — everything this symbol depends on.

**Usages** — every place this symbol appears, including aliases and re-exports.

---

## How it works

1. Graphyn parses your codebase using [tree-sitter](https://tree-sitter.github.io) — fast, incremental, no compiler needed
2. Builds a deterministic relationship graph (no LLM involved in graph construction)
3. Resolves aliases — `import { A as B }` is tracked across the entire codebase
4. Tracks property-level access — not just "uses class" but "accesses `.userId`"
5. Persists the graph to disk — sub-2s startup on any size codebase
6. Exposes everything via an MCP server — works with any MCP-compatible agent

```
Your codebase
      ↓  tree-sitter (language adapters)
   Intermediate Representation (IR)
      ↓  graphyn-core (Rust)
   Relationship graph (petgraph + DashMap)
      ↓  MCP server
   Codex / Cursor / Claude Code / Copilot / any agent
```

---

## Quick start

### Install

```bash
cargo install graphyn-cli
```

### Index your repo

```bash
graphyn analyze ./my-repo
```

This parses every TypeScript/JavaScript file, builds the relationship graph, and persists it to `.graphyn/db` in your repo root.

### Query from the CLI

```bash
# What breaks if I change UserPayload?
graphyn query blast-radius UserPayload

# Narrow to a specific file if the name is ambiguous
graphyn query blast-radius UserPayload --file src/models/user_payload.ts

# All usages including aliases
graphyn query usages UserPayload

# Full dependency tree
graphyn query deps UserPayload

# Show graph stats
graphyn status
```

### Connect to your agent

Start the MCP server:
```bash
graphyn serve --stdio
```

Or run in watch mode (live incremental updates as you code):
```bash
graphyn watch ./my-repo
```

---

## Agent integration

### Cursor

Add to `.cursor/mcp.json` in your project root:

```json
{
  "mcpServers": {
    "graphyn": {
      "command": "graphyn",
      "args": ["serve", "--stdio"],
      "env": {
        "GRAPHYN_ROOT": "${workspaceFolder}"
      }
    }
  }
}
```

### Claude Code

Add to `.claude/mcp_settings.json`:

```json
{
  "mcpServers": {
    "graphyn": {
      "command": "graphyn",
      "args": ["serve", "--stdio"]
    }
  }
}
```

### OpenAI Codex

Add to your Codex agent configuration:

```json
{
  "mcpServers": {
    "graphyn": {
      "command": "graphyn",
      "args": ["serve", "--stdio"],
      "env": {
        "GRAPHYN_ROOT": "."
      }
    }
  }
}
```

### Antigravity

Antigravity reads MCP servers from `.gemini/settings.json`:

```json
{
  "mcpServers": {
    "graphyn": {
      "command": "graphyn",
      "args": ["serve", "--stdio"],
      "cwd": "${workspaceFolder}"
    }
  }
}
```

### Any MCP-compatible agent

Graphyn speaks standard MCP over stdio. Any agent that supports MCP can connect:

```bash
graphyn serve --stdio
```

The server accepts `initialize`, `tools/list`, and `tools/call` — standard MCP JSON-RPC protocol.

### What your agent can now do

```
# Before changing a class, the agent calls:
get_blast_radius("UserPayload")

# Before deleting a function:
get_symbol_usages("processOrder")

# To understand what a module needs:
get_dependencies("AuthService")
```

The agent receives structured output it can act on — not a wall of code to search through.

---

## MCP tools

Graphyn exposes three tools via MCP:

| Tool | What it answers |
|---|---|
| `get_blast_radius` | What will break if I change symbol X? |
| `get_dependencies` | What does symbol X depend on? |
| `get_symbol_usages` | Where is symbol X used, including aliases? |

All three tools resolve aliased imports and track property-level access by default.

---

## Performance

| Operation | Target | Method |
|---|---|---|
| Query (blast radius) | < 100ms p95 | In-memory graph, no disk I/O on query path |
| Initial parse (50k LOC) | < 10s | Parallel file parsing with rayon |
| Incremental update (1 file) | < 500ms | Only re-parse changed file, diff graph |
| Startup (graph persisted) | < 2s | Deserialize from RocksDB |

The graph lives in memory. Queries traverse in-memory edges with no network or disk round-trip.

---

## Language support

| Language | Status |
|---|---|
| TypeScript | v1 — supported |
| JavaScript | v1 — supported |
| Python | v2 — planned |
| Rust | v2 — planned |
| Go | v2 — planned |
| Java | v2 — planned |

Adding a new language means writing a new adapter crate that outputs the [standard IR](docs/ir-spec.md). The core graph engine never changes.

---

## Building from source

```bash
git clone https://github.com/avax-labs/graphyn
cd graphyn
cargo build --release
```

Run all tests:
```bash
cargo test --workspace
```

---

## Status

**v0.1.0-beta** — TypeScript/JavaScript support is complete. All core features are implemented and tested.

- [x] Core graph engine with alias resolution
- [x] TypeScript/JavaScript adapter (tree-sitter)
- [x] Blast radius, dependencies, usages queries
- [x] Property-level access tracking
- [x] RocksDB persistence
- [x] Incremental file watching
- [x] MCP server (stdio)
- [x] CLI (analyze, query, watch, serve)
- [ ] Python adapter (v2)
- [ ] VS Code extension (post-beta)
- [ ] Web dashboard (post-beta)

---

## License

Apache 2.0 — see [LICENSE](LICENSE)