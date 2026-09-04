# Working on Graphyn

Guidance for Claude Code and other agents contributing to this repository.

Note this is *not* `agent-configs/claude/CLAUDE.md` — that file is a template
Graphyn ships to its users, telling their agent how to call Graphyn. This file
is about changing Graphyn itself.

## What Graphyn is

A deterministic symbol relationship graph for a repository, so developers and
coding agents can answer: what breaks if I change this, where is this used
including through aliases, and what does this depend on.

**The defining property is determinism.** No LLM participates in graph
construction. Identical input produces identical output, byte for byte. Every
other claim the product makes rests on that one, because you cannot gate CI on
an opinion.

## Non-negotiables

- **Determinism.** Any new collection that feeds output needs a stable ordering
  key. `HashMap` iteration order is seeded per process — use `BTreeMap`, or sort
  explicitly, anywhere the result reaches a user, a file, or a socket. This has
  regressed twice; it is the first thing to check in review.
- **No LLM in graph construction or in any gating decision.** Not a performance
  preference, the product's core property.
- **Honest output.** The README documents limits rather than listing aspirations.
  When a feature has a blind spot, document it in the PR that ships the feature.
  Never widen a claim past what the code does.

## Build and check

```bash
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Clippy warnings are errors. Every PR adds real tests — 0.2.0 deliberately
removed 27 `assert!(true)` stubs, so do not reintroduce that pattern.

### Golden snapshots

`crates/graphyn-cli/tests/golden/` records the full analysis of every fixture
project. A change there is not a failure — it is the diff you are being asked
to review — but it must never be invisible. After an intended change:

```bash
UPDATE_GOLDEN=1 cargo test -p graphyn-cli --test golden_ir
git diff crates/graphyn-cli/tests/golden
```

Read that diff before committing. For a change that claims to preserve
behaviour, an empty diff is the evidence; a non-empty one means the claim is
wrong or needs explaining in the PR.

## Dogfooding

Graphyn analyzing Graphyn is the best available test corpus, and a standing
integration test:

```bash
cargo run -- analyze .
cargo run -- query usages RepoIR
cargo run -- query blast-radius RepoIR
```

`RepoIR` is referenced across dozens of files. If `usages` returns nothing or
`blast-radius` reports "safe to modify", resolution is broken — that is a
regression, not a quirk.

## Conventions

- Conventional commit messages. One logical change per PR.
- Branch names derive from what the change does, with a conventional-commit
  type prefix: `fix/rust-workspace-crate-roots`, `feat/graph-delta`. No fixed
  scheme beyond that.
- Commit author email must be `jeelgajera200@gmail.com`.
- **Never** add session metadata, "Generated with" footers, or `Co-Authored-By`
  trailers to a commit message or PR body.
- Update `CHANGELOG.md` in the same PR, under `## [Unreleased]`, Keep a
  Changelog format.
- New fixtures go in `fixtures/`, following the existing polyglot layout.
- This repository is public. Do not commit scratch files, working notes, or
  task-tracking markdown — keep those local.

## Layout

| Crate | Role |
|---|---|
| `graphyn-core` | Graph engine, IR (`RepoIR`), symbol IDs, AST helpers, relationship model |
| `graphyn-adapter-dispatch` | Routes files to the owning adapter, runs groups in parallel, merges into one `RepoIR` |
| `graphyn-adapter-{ts,python,rust,go,c}` | Per-language parsing and resolution |
| `graphyn-store` | RocksDB persistence (`.graphyn/db`) |
| `graphyn-mcp` | MCP server (`serve --stdio`) |
| `graphyn-cli` | The `graphyn` binary |

Parsing is tree-sitter and is essentially a solved, vendored problem. **Quality
lives in resolution** — imports, aliases, and binding declared types for
property attribution. Every bug 0.2.0 fixed was a resolution bug, not a parse
bug. Budget accordingly when estimating work on a language.

## Documented limits

These are honest and stay honest. Do not quietly widen them:

- Imports resolve within one language only.
- Chained access is attributed to the first receiver only — in `a.b.c`, `c` is
  not attributed to the type of `a.b`.
- C++ templates are parsed but not instantiated.
- Go structural interface matching is per-package.
- Rust macro bodies are token trees; macro-generated code is not expanded.
- Fully-qualified paths used inline without a `use` record no edge.
