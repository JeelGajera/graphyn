# Graphyn MCP Tools Reference

This document describes the three MCP tools Graphyn exposes to coding agents. These tools are available to any MCP-compatible agent — Cursor, Claude Code, GitHub Copilot, or any custom agent built on the MCP protocol.

---

## Overview

| Tool | Purpose | When to use |
|---|---|---|
| `get_blast_radius` | What will break if I change symbol X? | Before modifying a class, function, or type |
| `get_dependencies` | What does symbol X depend on? | Before deleting a symbol or moving a module |
| `get_symbol_usages` | Where is symbol X referenced? | Before renaming a symbol |

All three tools:
- Resolve aliased imports (`import { A as B }`)
- Track property-level access (not just "uses class" but "accesses `.userId`")
- Work across the entire indexed codebase, including deep directories

---

## get_blast_radius

Returns all symbols that depend on the given symbol and would be affected by changes to it.

### Input schema

```json
{
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "The symbol name to analyze. e.g. 'UserPayload', 'authService', 'processOrder'"
    },
    "file": {
      "type": "string",
      "description": "Optional: narrow to a specific file if the symbol name appears in multiple files"
    },
    "depth": {
      "type": "integer",
      "description": "How many dependency hops to traverse. Default 3. Max 10.",
      "default": 3,
      "minimum": 1,
      "maximum": 10
    }
  },
  "required": ["symbol"]
}
```

### Example call

```json
{
  "symbol": "UserPayload",
  "depth": 3
}
```

### Example output

```
Symbol: UserPayload [class] — src/models/user_payload.ts:12

Blast radius (3 dependents, depth=3):

DIRECT (uses UserPayload by its original name):
  • src/handlers/auth.ts:45
    → imports as UserPayload
    → accesses: .userId, .email
    → context: "import { UserPayload } from '../models/user_payload'"

  • src/handlers/profile.ts:23
    → imports as UserPayload
    → accesses: .userId, .timestamp
    → context: "import { UserPayload } from '../models/user_payload'"

ALIASED (uses UserPayload under a different import name — high risk):
  • src/mappers/response/deep/view_model_mapper.ts:8
    → imports as ResponseModel  ← ALIAS
    → accesses: .userId, .timestamp, .status
    → context: "import { UserPayload as ResponseModel } from '../../../models/user_payload'"

Properties at risk if changed:
  .userId      — referenced in 3 files
  .timestamp   — referenced in 2 files
  .status      — referenced in 1 file (via alias only)
  .email       — referenced in 1 file
```

### When to call this

Call `get_blast_radius` before:
- Modifying a class definition (adding, removing, or renaming properties)
- Changing a function signature
- Modifying a type alias or interface
- Moving or renaming a module

The `ALIASED` section is the most important part. Text search and naive agents miss aliased imports. Graphyn does not.

---

## get_dependencies

Returns everything the given symbol depends on — its complete dependency tree.

### Input schema

```json
{
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "The symbol name to analyze."
    },
    "file": {
      "type": "string",
      "description": "Optional: narrow to a specific file."
    },
    "depth": {
      "type": "integer",
      "description": "How many dependency hops to traverse. Default 3.",
      "default": 3,
      "minimum": 1,
      "maximum": 10
    }
  },
  "required": ["symbol"]
}
```

### Example call

```json
{
  "symbol": "ViewModelMapper",
  "depth": 2
}
```

### Example output

```
Symbol: ViewModelMapper [class] — src/mappers/response/deep/view_model_mapper.ts:4

Dependencies (depth=2):

DIRECT:
  • src/models/user_payload.ts — UserPayload [class]
    → imported as ResponseModel (alias)
    → properties used: .userId, .timestamp, .status

  • src/utils/date_formatter.ts — formatTimestamp [function]
    → called directly

TRANSITIVE (depth 2):
  • src/models/base_model.ts — BaseModel [class]
    → extended by UserPayload
```

### When to call this

Call `get_dependencies` before:
- Deleting a module to understand what it needs from the rest of the codebase
- Understanding what an unfamiliar class or function requires
- Debugging an import error or circular dependency

---

## get_symbol_usages

Finds every location in the codebase where a symbol is referenced, including under aliases and re-exports.

### Input schema

```json
{
  "type": "object",
  "properties": {
    "symbol": {
      "type": "string",
      "description": "The symbol name to search for."
    },
    "file": {
      "type": "string",
      "description": "Optional: narrow to a specific file."
    },
    "include_aliases": {
      "type": "boolean",
      "description": "Include usages under aliased imports. Default true. Do not set false.",
      "default": true
    }
  },
  "required": ["symbol"]
}
```

### Example call

```json
{
  "symbol": "UserPayload"
}
```

### Example output

```
Symbol: UserPayload [class] — src/models/user_payload.ts:12

Usages (4 total, including 1 aliased):

  • src/handlers/auth.ts:45
    import { UserPayload } from '../models/user_payload'
    
  • src/handlers/profile.ts:23
    import { UserPayload } from '../models/user_payload'

  • src/mappers/response/deep/view_model_mapper.ts:8
    import { UserPayload as ResponseModel } from '../../../models/user_payload'
    ↳ used as "ResponseModel" throughout this file

  • src/index.ts:3
    export { UserPayload } from './models/user_payload'
    ↳ re-exported (barrel)
```

### When to call this

Call `get_symbol_usages` before:
- Renaming a symbol (need to update all references)
- Deleting a symbol (need to verify it is truly unused)
- Understanding the scope of a symbol's reach

---

## Usage pattern for agents

A well-instrumented agent should call Graphyn tools in this sequence before making any structural change:

```
1. get_blast_radius(symbol)
   → Understand the full impact

2. get_symbol_usages(symbol)
   → Find every file that needs to be updated

3. Make changes to all affected files

4. get_blast_radius(symbol) [again after changes]
   → Verify the blast radius is now what was expected
```

Agents that skip step 1 are the agents that cause the aliased import bug. Graphyn exists to make step 1 automatic and complete.

---

## Connecting to Graphyn

### Cursor

`.cursor/mcp.json` in project root:
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

`.claude/mcp_settings.json`:
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

### Generic MCP client

Graphyn speaks standard MCP over stdio. Start the server:
```bash
graphyn serve --stdio
```

Send a standard MCP `initialize` request, then call tools using the standard `tools/call` method.

---

## Error responses

When a symbol is not found in the graph:
```
Symbol "UnknownClass" not found in the indexed graph.

If this symbol exists in your codebase, run:
  graphyn analyze ./
to rebuild the index, then try again.
```

When the graph is not initialized:
```
No graph index found. Run `graphyn analyze ./` first.
```

These responses are human-readable and agent-actionable — the agent knows exactly what to do next.