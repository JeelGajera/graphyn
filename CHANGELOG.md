# Changelog

All notable changes to Graphyn are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **A C call through a header prototype reaches the definition.** C splits a
  call across two files, and the shape is the dominant one in plain C: the
  caller includes a header that *declares* the function, while the definition
  lives in a `.c` file the caller never sees. Callers now attach to the
  definition. That direction is the point — the graph answers "what breaks if I
  change this", and a caller attached to the declaration would leave
  `blast-radius` on the definition returning nothing, the exact failure call
  edges exist to prevent.

  The link is an agreement between two files rather than a name matched across
  the repository: a header declares `N`, and exactly one file both defines `N`
  and includes that header. Including the header that declares you is what a C
  build already does so the compiler can check the two agree, so the rule keys
  on a fact the language enforces rather than on a filename convention like
  `geometry.c` implementing `geometry.h`. Two definers make it ambiguous and a
  definer that does not include the header is unanchored; both record nothing
  rather than guess, and neither raises a diagnostic. A definition the caller
  can see directly still wins, so a `static` helper shadows an external
  function of the same name exactly as it does at compile time.

  A prototype is not a symbol. It names a function defined elsewhere, so
  minting a node for it would put two nodes in the graph for one function and
  make the name ambiguous to `find_symbol_id` — the ambiguity 0.2.0 spent a
  release removing. The placeholders exist only between extraction and
  resolution and are dropped before the graph is returned.


- **Call and instantiation edges for C and C++.** A bare `foo(..)` records a
  call when the name resolves to a function the file can see — its own, or one
  defined in a header it includes. `new Foo(..)` records `Instantiates`; C has
  no construction syntax at all, since `struct Foo f = {..}` is a declaration
  with an initializer rather than an expression naming a constructor. A C++
  functional cast, `Celsius(x)`, is spelled exactly like a call and calls
  nothing, so as elsewhere the resolved target's kind settles the kind.

  This is the narrowest of the five languages, and deliberately so. `ns::func()`
  and `obj->method()` record nothing: C++ methods are not symbols in this graph,
  only their class is, so an edge would either name the class — claiming the
  class was called — or match a leaf name repository-wide, which is the bug
  0.2.0 fixed elsewhere.


- **Call and instantiation edges for Go.** Go is the one language where the
  selector rule had to be inverted. `obj.method()` records no call edge in
  TypeScript, Python and Rust, but a cross-package call in Go is *always*
  written `pkg.Func(..)` — applying the same rule would have left Go with call
  edges that never cross a file boundary. So a selector call is recorded when
  its operand is one of the file's imported package names, and skipped when it
  is a value. The file's own import list is the only thing that tells the two
  apart, and it is already parsed.

  `Foo{..}`, `pkg.Foo{..}` and `&Foo{..}` are composite literals — Go's
  construction syntax — so they record `Instantiates` from the syntax alone. A
  composite literal for a slice, map or array names no symbol and records
  nothing.

  `models.UserID(42)` is a conversion spelled exactly like a call. Nothing is
  called, so as in Python the resolved target's kind settles the kind: a target
  that turns out to be a type makes the edge an instantiation. A `Calls` edge
  still only ever targets a function or a method.

  Builtins (`len`, `make`, `append`) and standard-library calls record nothing
  and raise no diagnostic, since neither names a symbol in the graph.


- **Call and instantiation edges for Rust.** Rust spells the cases differently
  and each is recorded as what it is. `Foo::new(..)` is an associated function
  call, so the edge names the *method* `Foo::new` rather than the type — `new`
  returns `Self` only by convention, and an edge to the type would leave
  `blast-radius` on the method blind to its own callers. `Foo { .. }` is Rust's
  actual construction syntax and records `Instantiates` outright. `Foo(1)` on a
  tuple struct reads as a plain call, so, as in Python, the resolved target's
  kind is what makes it an instantiation.

  `Enum::Variant(x)` is construction too, and is recorded as such: nothing ever
  calls a variant, and an edge saying otherwise would surface under
  `--kind calls` as a caller that does not exist. After both promotions a
  `Calls` edge only ever targets a function or a method — something that can
  actually run.

  A path callee resolves through the file's imports and then into the file that
  defines the type, never by matching a leaf name across the repository. A
  fully-qualified path used inline without a `use` still records no edge, which
  is the documented limit and is now pinned by a test rather than only by
  prose. `obj.method()` records nothing, `println!(..)` is a macro invocation
  rather than a call node, and prelude names bind to nothing — none of them
  raise a diagnostic, because there is nothing a user could fix.


- **Call and instantiation edges for Python.** `foo()` where `foo` was imported
  or defined in the file records a `Calls` edge to the definition, travelling
  through the same import machinery as every other reference, so a call through
  a renamed import resolves to the canonical symbol rather than the alias.

  Python spells construction and invocation identically — `Foo()` and `foo()`
  are the same node — so the split is made from the resolved target: an edge
  whose target turns out to be a class is promoted to `Instantiates`. Deciding
  it from a capitalized name would be a naming convention presented as an
  analysis.

  As in TypeScript, `obj.method()` records no call edge (it is already a
  property access on the receiver, which is the honest statement), and a callee
  that binds to nothing records nothing at all — no `print()` edge, and no
  diagnostic, because there is nothing a user could fix. A call landing on a
  third-party import is dropped for the same reason: the only id available is
  the package, and "this function calls the requests package" is a claim about
  a symbol nobody can open. The `Imports` edge already carries that dependency.


- **Per-edge resolution, and a safety verdict that depends on it.** Every
  relationship carries a `resolution` of `resolved` (bound through the file's
  imports, aliases and declared types) or `structural` (matched by name inside
  one file). A polyglot graph mixes both in one answer, so it belongs on the
  edge: `blast-radius` over a TypeScript and Java repository returns rows from
  each, and until now they read identically.

  `blast-radius` finding nothing prints "safe to modify" — a claim about the
  whole repository. It now holds that claim back when any edge in the graph is
  structural, naming the files that were analyzed within-file only, because a
  reference living in one of them would never have reached the graph. The
  result is still reported; what changes is that it is no longer presented as
  proof. Structural rows in a non-empty result are marked in both the CLI and
  the MCP context, since an agent acts on that line.

  Adapters do not set the field: dispatch stamps `Resolved` on Tier 1 output
  and structural analysis leaves the default. The default is the *weaker*
  value, so forgetting it under-claims rather than granting gate-safety.


- **Call and instantiation edges**, for TypeScript and JavaScript. `foo()`
  where `foo` arrived through an import or is defined in the file records a
  `Calls` edge to that function; `new Foo()` records `Instantiates`. Both
  travel through the existing import machinery, so a call to a function
  imported through a barrel chain resolves to the file that defines it.

  A callee that binds to nothing records **no edge at all** — not an edge
  against a placeholder. `setTimeout(...)`, a global, or a value from an
  untyped module names no symbol in the graph, and inventing one by matching
  the leaf name repository-wide is the bug 0.2.0 fixed in the Rust adapter.
  No diagnostic is raised for these, because there is nothing a user could fix.

  `obj.method()` is deliberately not a call edge. It is already recorded as a
  property access on the receiver's declared type, which is the honest
  statement; a `Calls` edge to the *type* would claim the type was called and
  would double every row for one source location.


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

- **A tier model, so "supports N languages" means something.** `LanguageSpec`
  describes a language and how much of it Graphyn can resolve. Tier 1
  (`resolved`) has full import, alias and declared-type resolution and is safe
  to gate on. Tier 2 (`structural`) has symbols and intra-file references only,
  and is advisory by construction — a gate can fail open on it automatically
  rather than reporting a pass it did not earn. `graphyn status` reports the
  tier of every language a build carries.

  Consolidating the adapters removed the packaging cost of a language, not the
  engineering cost. Import resolution, alias resolution and declared-type
  binding are irreducibly per-language and cost weeks; every bug 0.2.0 fixed
  was a resolution bug, not a parse bug. Tiers are how breadth stays honest.

- **Tier 2 needs no query files written for it.** Every tree-sitter grammar
  Graphyn vendors already ships `queries/tags.scm` and exposes it as
  `TAGS_QUERY`, using a standard capture vocabulary — `@definition.*`,
  `@reference.*`, `@name` — that maps onto Graphyn's own `SymbolKind` and
  `RelationshipKind`. One generic analyzer covers every structural language.
  Adding one is a dependency, a feature flag and a spec.

- **Java, as the first Tier 2 language** (`--features java`, not in `default`).
  Its whole module is a spec: no parser, no extractor, no resolver, no queries.
  It extracts classes, interfaces and methods and links intra-file calls and
  `implements`; it resolves nothing across files, and `status` says so.

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


- **Tier 2 languages resolve nothing across files.** A structural language
  records a reference only when the name is defined in the same file. A tags
  query reports that a call to `foo` happened; it does not say which `foo`, and
  matching by leaf name repository-wide is precisely the bug 0.2.0 fixed in the
  Rust adapter. "No usages found" in a Tier 2 file is a statement about that
  file.

- **Fully-qualified paths used inline are not resolved.** A type written as
  `graphyn_core::ir::RepoIR` in a signature, with no `use` bringing it into
  scope, records no edge — resolution binds names through the file's import
  table, and such a path never enters it. On this repository that accounts for
  21 of the 45 files mentioning `RepoIR`, and for the `Display` and `Error`
  references reported against `impl std::fmt::Display for …`. Pre-existing, and
  unrelated to crate-root discovery; documented here because the workspace fix
  is what made it visible.

- **Call and instantiation edges cover every Tier 1 language, at different
  depths.** C and C++ are the narrowest, and Tier 2 languages emit `Calls`
  within a file only. A `--kind calls` query against a Java repository therefore
  returns only within-file callers — and says so, naming the graph rather
  than the feature: the CLI and MCP tools report which kinds the analyzed graph
  actually contains instead of consulting a hand-maintained list of
  unimplemented kinds. That list was wrong twice inside one release, and could
  not express "call edges exist, but not for this repository's language" — which
  is exactly the statement that had to stay true as each language followed.

### Fixed

- **`graphyn-lang` did not build with only a Tier 2 language enabled.** With
  every Tier 1 language turned off, each arm of the dispatch `match` is compiled
  out and the sole remaining arm is `Vec::new()`, leaving an element type
  nothing pins down; the `java_structural` test additionally carried an unused
  import that only a java-enabled build ever compiled. `cargo build
  --no-default-features --features java` failed on both. Neither was reachable
  from `--workspace`, because java is not a default feature, so the file that
  broke was never compiled by CI.

  CI now runs clippy over `graphyn-lang` with each language feature enabled on
  its own, taking the list from the crate's own feature table so a new language
  is covered without anyone remembering to add it in two places. The
  contributing guide has asked for this check by hand since the languages
  became features; asking was not enough.

- **The snapshot format dropped the resolution of every edge.** Introduced and
  caught within this release: `GraphSnapshot` did not persist the new field, so
  a graph loaded from `.graphyn/db` — which is every query, since queries read
  the store — came back entirely `structural`. The failure was in the safe
  direction and therefore silent: nothing errored, but `blast-radius` refused
  to say "safe to modify" about a fully resolved repository, and the feature
  was useless without being visibly broken. Snapshot version is now 3; a
  version 1 or 2 snapshot reads back as `structural` and is rewritten on the
  next analyse.

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
