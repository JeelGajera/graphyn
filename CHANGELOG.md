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

- `--kind` on `query blast-radius`, `usages` and `deps`, restricting a
  traversal to named relationship kinds (`imports`, `extends`, `implements`,
  `uses-type`, `accesses-property`, `re-exports`). Repeatable. MCP query tools
  take the same filter as a `kinds` argument, and every returned edge now
  reports its kind.

  Filtering happens on the edge rather than on the collected result, so an
  excluded kind also stops the walk continuing through it: asking what imports
  a symbol will not return something that merely inherits from an importer.

  An unrecognised name is rejected rather than ignored, and a filtered empty
  result is never reported as "safe to modify" — the tool searched a slice of
  the graph, so the claim would be unearned.

- Golden snapshots of the analysis over the whole fixture corpus, and a CI
  step asserting that two analyses of identical input agree byte for byte.
  Per-adapter tests each assert one property of one language, which catches the
  regression someone thought to look for and misses a refactor that quietly
  changes the graph everywhere. Regenerate with `UPDATE_GOLDEN=1 cargo test -p
  graphyn-cli --test golden_ir` and review the diff.

### Changed

- **Ten crates became five.** The five `graphyn-adapter-*` crates and
  `graphyn-adapter-dispatch` are now one crate, `graphyn-lang`, with a module
  and a Cargo feature per language. `graphyn-core`, `graphyn-lang`,
  `graphyn-store`, `graphyn-mcp` and `graphyn-cli` remain.

  Three arguments usually justify a crate per language and none held here:
  every adapter depended on `graphyn-core` and was consumed only by dispatch,
  so they versioned in lockstep and had exactly one consumer; and the expensive
  compilation is the generated C in the upstream `tree-sitter-*` crates, which
  cargo caches regardless of how this code is laid out.

  The cost was real, though. The publish job had to push ten crates in
  dependency order, and a failure partway through leaves earlier crates on
  crates.io at a version that cannot be reused. Every new language lengthened
  that chain. Adding a language no longer touches the publish job at all.

  Under SemVer 0.x this is a permitted breaking change, and 1.0.0 is the last
  point at which the crate names are not yet a public commitment.

- The `graphyn-adapter-ts`, `-python`, `-rust`, `-go`, `-c` and `-dispatch`
  crate names are retired. A final patch release of each, carrying a
  deprecation notice pointing at `graphyn-lang`, should be published from the
  0.2.0 tag before 1.0.0 goes out. Nothing is yanked: yanking breaks existing
  lockfiles for no benefit.

- **`supported_languages()` reports what a build carries**, rather than a
  hardcoded list that happened to be accurate because only one build existed.

- Per-language features reach the binary: `cargo install graphyn-cli
  --no-default-features --features python` produces a working Python-only
  `graphyn` that indexes Python and skips other languages rather than failing
  on them. Measured on this machine, that binary is 16M against 27M for all six
  languages.

### Known limits

- **Fully-qualified paths used inline are not resolved.** A type written as
  `graphyn_core::ir::RepoIR` in a signature, with no `use` bringing it into
  scope, records no edge — resolution binds names through the file's import
  table, and such a path never enters it. On this repository that accounts for
  21 of the 45 files mentioning `RepoIR`, and for the `Display` and `Error`
  references reported against `impl std::fmt::Display for …`. Pre-existing, and
  unrelated to crate-root discovery; documented here because the workspace fix
  is what made it visible.

- `RelationshipKind::Calls` and `Instantiates` are declared but emitted by no
  adapter, so a filter naming them matches nothing. Rather than returning a
  silent empty result — which in a tool used to judge whether a change is safe
  reads as "nothing calls this" — both the CLI and the MCP tools say the kind
  is unimplemented. Call and instantiation edges are planned for 1.0.0.

### Fixed

- **`is_aliased_only_property` tested the wrong predicate.** It checked
  `alias.is_some()` while `partition_by_alias` had already been corrected to
  call `is_renamed` — one call site missed by commit `498333e`, duplicated
  verbatim in the CLI and the MCP server. Adapters record the local name on a
  reference whether or not it differs from the symbol's own, so an ordinary
  `import { UserPayload }` had every field it touched labelled "(aliased import
  only)". The predicate now lives in `graphyn-core` next to `is_renamed`, with
  both copies deleted.

  It also returned true for a property no edge touches, because `all` over an
  empty iterator is true — a claim about references that do not exist.

- **"imports as X" was printed for two opposite meanings.** The aliased branch
  rendered "renamed to X" and the fallback rendered "imported by X" using the
  identical phrase, so a reader could not tell which was meant — in the tool
  whose entire pitch is alias awareness. A genuine rename now reads "renamed
  to X"; a module-scope referrer, which was most of what the fallback emitted
  as "imports as module", prints nothing, since the location line already says
  where the reference is. Fixed in all three places it had been copied to.

- **One reference was reported as several dependents.** A source location
  attributed at both class and method level produces two edges differing only
  in `from`, and `from` was part of the deduplication key, so both survived. A
  64-file project reported 196 "dependents" for 38 referencing files, with the
  aliased findings below 160 rows of duplicates. `from` no longer splits rows;
  `kind` still does, because an `extends` and a field read at one line are two
  different facts. Sorting now happens before collapsing, so which row survives
  is decided by the ordering — lowest hop wins — rather than by traversal
  order.

- **The alias count counted symbols, not aliases.** `alias_chains` is keyed by
  symbol, so its length is the number of symbols that have aliases. Reported as
  "Alias chains", it read as a count of renames: a type imported under a
  different name by twelve files reported `1`. Both numbers are now reported —
  "Aliases 21 (across 19 symbols)" — and JSON gains an `aliases` field
  alongside `alias_chains` rather than redefining it, since adding a field is
  not a contract change and redefining one is.

- **Rust resolution assumed a single crate rooted at `<repo>/src/`.** In a
  Cargo workspace nothing resolved: `crates/graphyn-core/src/ir.rs` was read as
  the module `crate::crates::graphyn-core::src::ir`, so every `crate::` path
  failed and every `use graphyn_core::…` was classified as a third-party crate.
  Graphyn could not analyze itself — `query usages RepoIR` found nothing and
  `blast-radius RepoIR` reported a type referenced across the workspace as safe
  to modify. It now finds 36 usages and 42 dependents.

  The same assumption broke *nested* crates that are not workspace members,
  including this repository's own Rust fixtures, which were folded into one
  imaginary crate alongside the real code.

  Crate roots are now discovered per package — `[workspace] members` with globs
  and `exclude` honoured, `[lib]`/`[[bin]]`/`[[test]]`/`[[bench]]`/`[[example]]`
  paths read, and Cargo's conventional locations used as a fallback. Module
  paths are namespaced by the crate that owns them, so `crate::ir` inside
  `graphyn-core` and `graphyn_core::ir` written anywhere else resolve to one
  place, and intra-crate and inter-crate resolution become the same lookup
  rather than two mechanisms that have to agree. Package names normalise
  (`graphyn-core` → `graphyn_core`), `[lib] name` overrides are honoured, and
  dependency renames (`foo = { package = "bar" }`) are followed.

  A tree with no `Cargo.toml` still resolves: a `src/lib.rs` is a crate root
  whether or not a manifest sits beside it. Requiring one would have silently
  reclassified such paths as third-party rather than reporting them unresolved,
  and a wrong answer with no diagnostic is the worst available outcome.

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
