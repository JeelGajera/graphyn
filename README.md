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

Cargo (crates.io):
```bash
cargo install graphyn-cli
```

From source:
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

| Language | Extensions | Resolves |
| --- | --- | --- |
| TypeScript / JavaScript | `.ts` `.tsx` `.js` `.jsx` `.mts` `.cts` `.mjs` `.cjs` | `tsconfig` paths, barrel re-exports, decorator DI |
| Framework files | `.vue` `.svelte` `.astro` | script blocks within the component |
| Python | `.py` `.pyi` | relative imports, `__init__` re-export chains, Pydantic / Django / dataclass fields |
| Rust | `.rs` | module tree, `use` groups and aliases, trait impls, `#[derive]` |
| Go | `.go` | package imports via `go.mod`, structural interface satisfaction |
| C | `.c` `.h` | `#include` resolution, `typedef` aliases |
| C++ | `.cpp` `.cc` `.cxx` `.hpp` `.hxx` `.hh` | `using` aliases, base classes, namespace-qualified names |

Every adapter resolves import aliases and attributes member access to the type
a value was declared as, so `payload.user_id` is recorded against `UserPayload`
however the variable was named.

### Known limits

Being explicit about these is more useful than a feature list:

- **Imports resolve within one language.** A Python module importing a
  TypeScript file through a build step is not linked.
- **Chained access is attributed to the first receiver only.** In `a.b.c`, the
  `c` is not attributed to the type of `a.b`, which would require field-type
  resolution.
- **C++ templates are parsed but not instantiated.** `vector<Foo>` records a
  reference to `Foo`; it does not model what the instantiation generates.
- **Go structural matching is per-package.** A type satisfying an interface
  declared in another package is not reported, because matching every method
  set against every interface repository-wide produces far more noise than
  signal.
- **Rust macro bodies are token trees.** Field access inside `format!` and
  friends is recovered by scanning tokens; more elaborate macro-generated code
  is not expanded.

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

## Changelog

Release history is in [CHANGELOG.md](CHANGELOG.md).

## License

Apache-2.0 — see [LICENSE](LICENSE)
