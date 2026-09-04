# Changelog

All notable changes to Graphyn are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `graphyn analyze --json` — emits the analysis as a machine-readable document
  on stdout instead of a human summary. Carries a `schema_version` so a
  consumer can tell a shape it understands from one it does not; the version is
  present from the first release that emits JSON at all, because retrofitting
  one forces every existing consumer to handle its absence.

### Fixed

- **Serialized output was not reproducible.** `RepoIR.language_stats` was a
  `HashMap`, and the dispatch layer built it as an ordered `BTreeMap` only to
  collect it straight back into a `HashMap`, discarding the ordering it had
  just established. `HashMap` iteration order is seeded per process, so the
  same analysis emitted its language counts in a different order on every run.
  Nothing surfaced it before because no output path serialized the field.
  Determinism is the property the graph is built to guarantee, and it has to
  hold all the way out to the bytes on stdout.

## [0.2.0] - 2026-08-16

Graphyn is no longer TypeScript-only. This release adds Python, Rust, Go and
C/C++ adapters behind a dispatch layer, and grounds property resolution in
declared types across every adapter.

### Added

- **Python adapter** (`graphyn-adapter-python`) — `.py` and `.pyi`. Relative
  imports, `__init__.py` re-export chains, star imports, `TYPE_CHECKING`
  guards, and Pydantic / Django / dataclass field attribution.
- **Rust adapter** (`graphyn-adapter-rust`) — `.rs`. Module tree from `mod`
  declarations, `use` groups and aliases, glob imports, trait impls,
  `#[derive(...)]`, and field access recovered from macro token trees.
- **Go adapter** (`graphyn-adapter-go`) — `.go`. Package imports resolved
  against `go.mod` discovered per directory, and structural interface
  satisfaction from method sets.
- **C/C++ adapter** (`graphyn-adapter-c`) — `.c` `.h` `.cpp` `.cc` `.cxx`
  `.hpp` `.hxx` `.hh`. `#include` resolution, `typedef` aliases, C++ `using`
  aliases, base classes and namespace-qualified names.
- **Dispatch layer** (`graphyn-adapter-dispatch`) — routes files to the adapter
  that owns their language, runs adapter groups in parallel, and merges the
  results into one `RepoIR`. `supported_languages()` reports what a build can
  analyse.
- Shared symbol-id and AST traversal helpers in `graphyn-core`, replacing five
  per-adapter copies.
- Adapters emit parse, resolution and skip diagnostics; pruned directories are
  reported.
- Cross-language regression suite and polyglot fixtures. Test count went from
  39 to 185, replacing 27 `assert!(true)` stubs with real coverage.

### Changed

- The CLI and MCP server now call `graphyn-adapter-dispatch` instead of
  `graphyn-adapter-ts` directly. Mixed-language repositories index in one pass.
- Dispatch groups files by a stable ordering key and sorts each group, so graph
  construction order is reproducible across runs. `HashMap` iteration order had
  made two runs over identical input produce differently-ordered output.
- AST traversal is iterative and depth-bounded. Recursion over deeply nested
  input could overflow a rayon worker's stack and abort the process.
- Unresolved edges are removed with a warning instead of being silently dropped
  during graph construction.
- `env`, `gen` and `proto` are indexable again, and an explicit `--include` now
  overrides the built-in excludes.
- `README.md` documents each adapter's known limits rather than listing
  aspirational features.

### Fixed

- **Property access resolved against a hardcoded variable name.** The new
  adapters compared the receiver against the literal string `"data"` — the name
  used in every fixture — so property tracking returned nothing for any other
  name in Rust, Go and C/C++. Receivers are now bound to declared types from
  parameters, `let`/`var` bindings, annotations and declarators, and properties
  are keyed per resolved type. One struct no longer inherits another's fields.
- **`#include` produced a graph with zero edges.** Includes became
  `local_header::<name>`, which `add_relationship` drops. They now resolve to
  the header's module symbol.
- **Every C struct name reported as ambiguous.** A `struct Foo *p` parameter is
  also a `struct_specifier`; treating it as a definition minted one symbol per
  referencing file. Structs and classes are recorded only where defined.
- Go imports pointed at an arbitrary member instead of the package; they now
  target a synthetic package symbol, and qualified references carry the
  specific type.
- Python indexed only some symbol kinds, so importing a function fabricated an
  `ext::<module>::package` dependency that did not exist.
- Rust use-paths matched leaf names repository-wide instead of resolving
  through the module that declares the name and following `pub use` chains.
- Go structural interface detection read method sets from the symbol table
  instead of splitting signature text on punctuation.
- An alias equal to the symbol's own name is no longer reported as a rename,
  which had buried genuine renames among ordinary references.
- The crates.io publish job now publishes all ten crates in dependency order,
  and the five new crates carry the metadata the registry requires. The release
  would previously have failed partway through, after `graphyn-core` was
  already published.

### Known limits

- Imports resolve within one language. A Python module importing a TypeScript
  file through a build step is not linked.
- Chained access is attributed to the first receiver only. In `a.b.c`, `c` is
  not attributed to the type of `a.b`.
- C++ templates are parsed but not instantiated.
- Go structural matching is per-package.
- Rust macro bodies are token trees; macro-generated code is not expanded.

## [0.1.3] - 2026-05-09

### Added

- Dependency-injection tracking and parallel parsing in `graphyn-core`.

### Fixed

- Windows path handling.

### Changed

- CI drops Intel macOS binaries; installers enforce Apple Silicon.

## [0.1.2] - 2026-04-28

### Added

- ESM and framework file support (`.vue`, `.svelte`, `.astro` script blocks).
- Durable re-export persistence.

### Changed

- Installer UX improvements and simplified `tsconfig` path sorting.

## [0.1.1] - 2026-04-18

### Added

- Cross-platform release pipeline and one-line installers for macOS, Linux and
  Windows.
- Crate publishing workflow.

## [0.1.0] - 2026-04-12

Initial release.

### Added

- `graphyn-core` graph engine, `graphyn-adapter-ts` TypeScript/JavaScript
  adapter, `graphyn-store` RocksDB persistence, `graphyn-mcp` MCP server and
  the `graphyn` CLI.
- `analyze`, `query blast-radius`, `query usages`, `query deps`, `status`,
  `watch` and `serve --stdio`.
- Configurable scan filtering and MCP `refresh_graph`.

[0.2.0]: https://github.com/JeelGajera/graphyn/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/JeelGajera/graphyn/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/JeelGajera/graphyn/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/JeelGajera/graphyn/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/JeelGajera/graphyn/releases/tag/v0.1.0
