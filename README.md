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
- `graphyn analyze <path> --json`: emit the analysis as JSON on stdout
- `graphyn watch <path>`: keep graph in sync on file changes
- `graphyn query blast-radius <symbol> [--file <path>] [--depth <n>] [--kind <kind>]`
- `graphyn query usages <symbol> [--file <path>] [--kind <kind>]`
- `graphyn query deps <symbol> [--file <path>] [--depth <n>] [--kind <kind>]`
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

## Filtering by relationship kind

Every query can be narrowed to particular kinds of reference:

```bash
# only what imports it, not what merely inherits from an importer
graphyn query blast-radius UserPayload --kind imports

# repeatable
graphyn query usages UserPayload --kind imports --kind re-exports
```

Kinds: `imports`, `extends`, `implements`, `uses-type`, `accesses-property`,
`re-exports`.

Filtering applies to the traversal, not to the result, so an excluded kind
also stops the walk continuing through it. A filtered query reports the filter
it used, and an empty filtered result is never described as safe — only part
of the graph was searched.

`calls` and `instantiates` are accepted names but no adapter emits them yet, so
they match nothing; the tool says so rather than returning a silent empty
result.

## Machine-readable output

`graphyn analyze --json` writes the full analysis to stdout as a single JSON
document and suppresses all progress output, so it can be piped directly:

```bash
graphyn analyze . --json > analysis.json
```

The document carries a `schema_version`. Pin it: fields may be added within a
version, and anything a consumer could observe breaking bumps it.

Output is deterministic — the same input produces byte-identical bytes, which
is what makes two analyses safe to diff.

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
| Rust | `.rs` | Cargo workspaces and per-crate module trees, `use` groups and aliases, trait impls, `#[derive]` |
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
- **Fully-qualified paths used inline are not resolved.** A type written out in
  place — `graphyn_core::ir::RepoIR` in a signature, with no `use` bringing it
  into scope — records no edge. Resolution binds names through a file's import
  table, and a path like that never enters it. Bring the type into scope with a
  `use` and it resolves normally.

Planned:
- Java / Kotlin
- Ruby
- PHP

## Slim builds

A default `graphyn` carries every supported language. To build only what you
need:

```bash
cargo install graphyn-cli --no-default-features --features python
```

Features: `typescript` (includes JavaScript), `python`, `rust`, `go`, `c`
(includes C++). `graphyn status` and `--help` report what your build can
analyse; a build skips files in languages it does not carry rather than
failing on them.

Measured on one machine, a Python-only binary is 16M against 27M for all six
languages.

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
