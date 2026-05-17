# Graphyn

Understand the blast radius before you change code.

Graphyn builds a deterministic symbol relationship graph for your repository so you and your coding agents can answer:
- What breaks if I change this symbol?
- Where is this symbol used (including aliases)?
- What does this symbol depend on?

## Why Graphyn

- Alias-aware: resolves `import { A as B }`
- Property-aware: tracks accessed members (for safer refactors)
- Deterministic: no LLM in graph construction
- Fast queries: in-memory graph traversal
- Agent-ready: MCP server for Cursor, Claude Code, Codex, and others

## Install

macOS / Linux:
```bash
curl -fsSL https://raw.githubusercontent.com/JeelGajera/graphyn/master/install.sh | bash
```

Windows (PowerShell):
```powershell
irm https://raw.githubusercontent.com/JeelGajera/graphyn/master/install.ps1 | iex
```

From source (Cargo):
```bash
cargo install graphyn-cli --git https://github.com/JeelGajera/graphyn
```

## Quick Start

1. Index a repo:
```bash
graphyn analyze ./my-repo
```

2. Run queries:
```bash
# impact analysis
graphyn query blast-radius UserPayload

# usages (alias-aware)
graphyn query usages UserPayload

# dependency tree
graphyn query deps UserPayload

# graph summary
graphyn status
```

3. Keep graph updated while coding:
```bash
graphyn watch ./my-repo
```

## Core Commands

- `graphyn analyze <path>`: parse and build graph into `.graphyn/db`
- `graphyn watch <path>`: keep graph in sync on file changes
- `graphyn query blast-radius <symbol> [--file <path>] [--depth <n>]`
- `graphyn query usages <symbol> [--file <path>]`
- `graphyn query deps <symbol> [--file <path>] [--depth <n>]`
- `graphyn status`: graph stats and coverage
- `graphyn serve --stdio`: start MCP server

## Filtering

Graphyn honors `.gitignore` by default. If a symbol is missing, check whether it
lives in an ignored folder such as `dist/`, generated output, or scratch files.

Override filters when needed:

```bash
graphyn analyze . --no-gitignore
graphyn analyze . --include "src/**/*.ts"
graphyn analyze . --exclude "tests/**"
graphyn watch . --include "packages/api/**/*.ts"
```

For MCP clients, `refresh_graph` accepts:

- `path`
- `respect_gitignore`
- `include`
- `exclude`

Example:

```json
{
  "path": ".",
  "respect_gitignore": false,
  "include": "src/**/*.ts",
  "exclude": "tests/**"
}
```

## MCP Integration

Start server:
```bash
graphyn serve --stdio
```

Agent and MCP setup templates are in [`agent-configs/`](agent-configs/).

The folder includes ready-to-use examples for:
- `AGENTS.md`
- Claude Code `CLAUDE.md`
- Claude Code Skills
- Cursor rules
- GitHub Copilot instructions
- Gemini guidance
- Antigravity-style rules/workflows
- MCP configs for Cursor, Claude Code, Antigravity and Codex

## Language Support

Supported now:
- TypeScript / JavaScript (`.ts`, `.tsx`, `.js`, `.jsx`, `.mts`, `.cts`, `.mjs`, `.cjs`)
- Framework files: Vue (`.vue`), Svelte (`.svelte`), Astro (`.astro`)
- Python (`.py`, `.pyi`) — including dataclass DI, Pydantic, Django, FastAPI
- Rust (`.rs`) — including trait bounds, derive macros, module tree resolution
- Go (`.go`) — including implicit interface detection, struct embedding
- C (`.c`, `.h`) — including typedef aliases, header/source split
- C++ (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx`, `.hh`) — including using aliases, templates, inheritance

Planned:
- Java / Kotlin
- Ruby
- PHP

## Build & Test

```bash
cargo build --release
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## License

Apache-2.0 — see [LICENSE](LICENSE)
