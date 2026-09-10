# Graphyn

Deterministic guardrails for agentic code changes.

Your agent will change seventeen symbols it did not know it was touching.
Graphyn tells it, or stops it.

It builds a deterministic symbol relationship graph for your repository, and
answers the questions an agent has to get right before it writes:

- **What breaks if I change this?** — every caller, including aliased ones.
- **Which tests cover it?** — so the verify loop is cheap enough to actually run.
- **Does this change break a rule this repository wrote down?** — and fail CI if so.

No model participates in any of it.

## Why Graphyn

- **Deterministic: no LLM participates in graph construction, or in any gating
  decision.** Identical input produces identical output, byte for byte. This is
  first because everything else depends on it — *you cannot gate CI on an
  opinion*. A tool that answers "what breaks if I change this" probabilistically
  can advise, but it cannot block a merge, because a check that returns a
  different answer on a re-run is a check nobody can act on. Every competing
  tool is probabilistic somewhere in that path.
- **Honest about what it does not know.** Coverage is published, per language,
  on every run; gates fail open on regions Graphyn cannot resolve rather than
  reporting a pass they have not earned. See [Resolution
  coverage](#resolution-coverage).
- Alias-aware: resolves `import { A as B }`, so a rename finds the callers a
  text search misses.
- Property-aware: attributes `payload.user_id` to the type the value was
  declared as.
- Fast queries: in-memory graph traversal.
- Agent-ready: hooks, an MCP server, and a GitHub Action — push as well as pull.

### Resolution coverage

An enforcement tool that tells you what it could **not** resolve is worth more
than one implying completeness, so `graphyn status` publishes the figure on
every run — overall and per language.

Measured on this repository, with the `full` binary:

```
Resolved           99.7% (4844 of 4860 edge(s))
  C#               0.0% of 2 edge(s)
  C/C++            100.0% of 32 edge(s)
  Go               100.0% of 40 edge(s)
  Java             0.0% of 2 edge(s)
  Python           100.0% of 33 edge(s)
  Ruby             0.0% of 12 edge(s)
  Rust             100.0% of 4663 edge(s)
  TypeScript       100.0% of 76 edge(s)
  16 edge(s) are structural: matched by name within one file.
  A gate must not act on them.
```

The 0.0% rows are Tier 2 languages, and they are the point: those sixteen edges
are named rather than absorbed into a headline. The default binary reports
100.0% of 4,844 edges for the same repository — not because it resolves more,
but because it does not carry those languages and never looks at their files.
Both figures are true; only one of them would be misleading on its own.

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
- `graphyn impact <file>`: what depends on one file
- `graphyn diff --base <rev> --head <rev>`: what changed between two snapshots
- `graphyn check [--diff-only]`: enforce `.graphyn/rules.toml`
- `graphyn tests <symbol> | --diff`: which tests exercise a symbol or a change
- `graphyn audit [--base <rev>] [--head <rev>]`: detect reward-hacking in a change
- `graphyn context <symbol> [--budget <tokens>]`: a minimal working set for orienting
- `graphyn report --base <rev> --head <rev>`: one markdown report for a PR comment
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

Kinds: `imports`, `calls`, `extends`, `implements`, `uses-type`,
`accesses-property`, `re-exports`, `instantiates`, `tests`.

Filtering applies to the traversal, not to the result, so an excluded kind
also stops the walk continuing through it. A filtered query reports the filter
it used, and an empty filtered result is never described as safe — only part
of the graph was searched.

`tests` is derived rather than parsed: a test file's references into non-test
code are restated under it, so `--kind tests` answers "what covers this" as a
filter over the graph rather than a separate traversal. The underlying `calls`
or `imports` edge is kept as well, so a query for callers still finds tests.

Which kinds a given repository contains depends on its languages. A filter
matching no edge in the analyzed graph is reported as such rather than
returning a silent empty result.

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

## Context

```bash
graphyn context RepoIR
graphyn context --diff --base HEAD --head worktree --budget 2000
```

Emits the symbol, what it depends on, what depends on it, and a signature for
each — the shape of the neighbourhood rather than its contents — and reports
what that cost against reading the same files whole.

**Measured on this repository**, and worth reading with its caveat:

| Question | Estimated tokens |
|---|---|
| `graphyn context RepoIR` | ~420 |
| Reading those 30 files whole | ~57,900 (**138x** more) |
| `rg -n RepoIR` | ~4,050 (10x more) |
| `rg -l RepoIR` — the file list alone | ~670 (**1.6x** more) |

The 138x figure is real but flatters the feature. An agent orienting itself
does not read thirty files whole; it runs a text search. Against `rg -l`, which
answers the same "which files touch this" question, the saving is 1.6x — and
the part a text search cannot produce, the signature skeleton, is delivered for
only 1 of those 31 entries, because inbound edges to a widely-used type are
attributed to each file's module symbol rather than to the function that uses
it.

So: useful for a narrow neighbourhood where signatures land, thin for a
widely-imported type. This is the capability that erodes as native code search
improves, and it is deliberately last in the release for that reason.

Token figures are byte-based estimates, not a tokenizer's count. Graphyn
vendors no tokenizer: one is model-specific, and a figure that moved with
somebody's model would not be reproducible.

A `--budget` drops the outermost hops first and reports how many symbols it
omitted, rather than truncating silently.

## Rules

A repository states its own constraints in `.graphyn/rules.toml`, and
`graphyn check` enforces them.

```toml
[[rule]]
name = "core-must-not-depend-on-cli"
kind = "forbid-dependency"
from = "crates/graphyn-core/**"
to   = "crates/graphyn-cli/**"
severity = "error"          # error (default) or warn

[[rule]]
name = "payload-is-stable"
kind = "no-field-removal"
symbol = "UserPayload"

[[rule]]
name = "god-node"
kind = "max-fan-in"
threshold = 60
severity = "warn"
```

| Kind | Fields | Fires when |
|---|---|---|
| `forbid-dependency` | `from`, `to` | An import or re-export runs from one path glob to another |
| `forbid-reference` | `from`, `to` | *Any* reference does — the superset, so "you may call into this but not import it" is expressible |
| `no-field-removal` | `symbol` | A named symbol loses a field. Needs a change, so pass `--base`/`--head` |
| `max-fan-in` | `threshold` | A symbol exceeds that many inbound references. Third-party packages are not counted |

`severity` defaults to `error`: a rule written without one is a rule someone
means to enforce, and defaulting to advisory would make every unannotated rule
silent.

Everything that can be wrong is wrong at parse time — an unknown kind, a glob
that does not compile, a missing field, a threshold of zero, two rules under one
name. A typo would otherwise sit in a repository until the day it was supposed
to catch something, and a gate that quietly enforces four of your five rules
reports a pass it has not earned.

```bash
graphyn check .                # rules that need only the current graph
graphyn check . --diff-only    # also the ones that need a change
```

**A rule has three outcomes, not two.** `forbid-dependency` is a claim about
*absence*, so reporting it satisfied claims every edge in scope was examined.
Where weaker edges could hide a violation the verdict is **undecided** — never
a pass, and never a failure either. That is how a Tier 2 region fails open
instead of passing quietly. Exit `0` clean, `1` a rule broken on resolved
evidence, `2` the check could not run at all.

## Audit

```bash
graphyn audit --base HEAD --head worktree
```

Detects changes that look like they were made to pass a check rather than to
work. Deterministic — no model is involved, here or anywhere else.

| Detector | Signal | Severity |
|---|---|---|
| `test-tampering` | A test stopped covering a symbol that changed in the same diff | error |
| `contract-erosion` | A symbol was removed while a surviving caller still referred to it | error |
| `dead-on-arrival` | A new symbol only tests refer to | warn |

An audit finding is an accusation, so the output is built to be checked rather
than believed. Every finding carries its evidence and the id you would write
down to suppress it. Detectors that did **not** run are named with the reason,
because an absent check otherwise reads as a passing one — and only files a
Tier 1 adapter resolved are in scope, since a name matched inside one file
cannot support an accusation.

Record a deliberate exception in `.graphyn/audit-ignore`:

```
test-tampering-1a2b3c4d  # the test was rewritten when the API changed
```

Suppressed findings are still shown, and a suppression that matches nothing is
reported as stale. Exit 0 when nothing was found at the requested severity, 1
when something was, 2 when the audit could not run.

## Test impact

```bash
graphyn tests UserPayload
graphyn tests --diff --base HEAD --head worktree
```

Returns the tests that exercise a symbol, or everything a change touched, so a
verify loop can run those instead of the whole suite or nothing at all.

This answer licenses an omission — naming a subset is a claim that the tests
left out cannot fail — so the confidence is carried by the exit status rather
than only printed:

| Exit | Means |
|---|---|
| `0` | Every changed symbol is reached by a resolved test edge; run this subset |
| `3` | Tests were found, but something could be missing; run the full suite |
| `2` | The question could not be answered |

A selection is incomplete when a changed symbol is reached by no recognized
test, when a test was selected on structural evidence, or when the repository
has structural regions at all — a test in one of those could exercise the
change without recording an edge.

Tests the change itself modified are reported separately and never counted as
coverage. A diff that edits a function and its only test is exactly where a
reviewer most needs to be told something is missing.

## Agent Hooks

MCP is pull-only: the agent has to decide to ask. Hooks are push — the graph
reaches the agent at the moment of the edit.

```bash
mkdir -p .claude/hooks
cp agent-configs/hooks/claude/*.sh .claude/hooks/
cp agent-configs/hooks/lib/graphyn-hook-lib.sh .claude/hooks/
chmod +x .claude/hooks/*.sh
# then merge agent-configs/hooks/claude/settings.json into .claude/settings.json

cp agent-configs/hooks/git/pre-commit  .git/hooks/pre-commit
cp agent-configs/hooks/git/post-commit .git/hooks/post-commit
cp agent-configs/hooks/lib/graphyn-hook-lib.sh .git/hooks/
chmod +x .git/hooks/pre-commit .git/hooks/post-commit

graphyn analyze . --snapshot HEAD
```

Before an agent edits a file, its blast radius is put into context:

```
Graphyn blast radius for src/models/user_payload.ts: 1 file(s) and 3
reference(s) depend on symbols defined here.
```

And a change that breaks a rule does not reach a commit:

```
$ git commit -m "drop unused email field"
    payload-is-stable no-field-removal [error]
        src/models/user_payload.ts:5
          field 'email' removed from 'UserPayload'

graphyn: commit blocked by a rule in .graphyn/rules.toml
         Override once with: git commit --no-verify
```

Only a rule violated on resolved evidence blocks. A missing binary, a missing
graph, a stale snapshot or a timeout all let the commit through and say on
stderr that nothing was checked — a gate that silently stops working is worse
than one that is plainly off. Full details, including the `post-commit` hook
that keeps the baseline fresh, are in
[`agent-configs/hooks/README.md`](agent-configs/hooks/README.md).
## GitHub Action

```yaml
# .github/workflows/graphyn.yml
name: Graphyn
on: pull_request
permissions:
  contents: read
  pull-requests: write
jobs:
  graphyn:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0   # both sides of the comparison must exist locally
      - uses: JeelGajera/graphyn@v0
```

It analyzes the base and head commits, compares them, evaluates
`.graphyn/rules.toml`, and posts one comment — updating it on each push rather
than adding another.

```markdown
## Graphyn

**1 rule(s) violated.** This change is blocked.

### Rules

| Rule | Kind | Verdict |
|---|---|---|
| core-must-not-depend-on-cli | forbid-dependency | **FAIL** |
| payload-is-stable | no-field-removal | pass |

**core-must-not-depend-on-cli** (forbid-dependency, error):

- `crates/graphyn-core/src/ir.rs:12` — core/ir.rs -> cli/main.rs (imports)
```

The verdict is the first line, so a reader who stops there still has the
answer. Rules that could not be decided are named in the comment rather than
only in the exit status, and never fail the job.

| Input | Default | Notes |
|---|---|---|
| `version` | `latest` | A release tag, or `source` to build from the checkout |
| `base-ref` | PR base | What to compare against |
| `rules` | `.graphyn/rules.toml` | |
| `comment` | `true` | |
| `fail-on-violation` | `true` | Undecided rules never fail the job |

## MCP Integration

Start server:
```bash
graphyn serve --stdio
```

Six tools, deliberately. A large tool surface degrades an agent's ability to
pick the right one, so each of these answers a question the others cannot:

| Tool | Answers |
|---|---|
| `get_blast_radius` | What breaks if I change this symbol? |
| `get_dependencies` | What does this symbol depend on? |
| `get_symbol_usages` | Where is this used, including under aliases? |
| `graph_diff` | What did this change break? |
| `check_rules` | Does this change violate the rules this repository wrote down? |
| `refresh_graph_index` | Re-analyze after changes |

The three query tools take `kinds` and `min_resolution` filters. Results carry
their resolution: an answer marked structural was matched within one file and
cannot see across files, so an empty result from it is not evidence that
nothing depends on the symbol. `graph_diff` and `check_rules` read snapshots
recorded by `graphyn analyze --snapshot` and never re-analyze, so their answers
depend only on the revisions named.

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

| Language | Tier | Extensions | Resolves |
| --- | --- | --- | --- |
| TypeScript / JavaScript | 1 | `.ts` `.tsx` `.js` `.jsx` `.mts` `.cts` `.mjs` `.cjs` | `tsconfig` paths, barrel re-exports, decorator DI |
| Framework files | 1 | `.vue` `.svelte` `.astro` | script blocks within the component |
| Python | 1 | `.py` `.pyi` | relative imports, `__init__` re-export chains, Pydantic / Django / dataclass fields |
| Rust | 1 | `.rs` | Cargo workspaces and per-crate module trees, `use` groups and aliases, trait impls, `#[derive]` |
| Go | 1 | `.go` | package imports via `go.mod`, structural interface satisfaction |
| C | 1 | `.c` `.h` | `#include` resolution, `typedef` aliases |
| C++ | 1 | `.cpp` `.cc` `.cxx` `.hpp` `.hxx` `.hh` | `using` aliases, base classes, namespace-qualified names |

Tier 2, behind their own features and not in `default`:

| Language | Tier | Extensions |
| --- | --- | --- |
| Java | 2 | `.java` |
| Ruby | 2 | `.rb` |
| C# | 2 | `.cs` |

Every Tier 1 adapter resolves import aliases and attributes member access to
the type a value was declared as, so `payload.user_id` is recorded against
`UserPayload` however the variable was named.

**What Tier 2 gives you, plainly.** Symbols, and references *within a single
file*. You can locate a definition, list what a file declares, and see
intra-file usage.

**What it does not.** No import resolution, no aliases, no declared types — so
no cross-file reference of any kind. A tags query reports that a call to `foo`
happened; it does not say which `foo`, and guessing by name across a repository
is the bug Graphyn exists to avoid.

**Gates do not fire on Tier 2 regions.** Not "are discouraged from" — they do
not. `check` reports an undecided verdict rather than a pass, `tests` refuses to
call its selection complete, and every audit detector is restricted to files a
Tier 1 adapter resolved, with the framework dropping an out-of-scope finding
even when a detector forgets to check. A structural region cannot support a
conclusion drawn from the *absence* of a reference, because the reference would
never have reached the graph.

### Known limits

Being explicit about these is more useful than a feature list:

- **An empty blast radius is only "safe to modify" when the whole graph is
  resolved.** Tier 2 (structural) analysis sees one file at a time, so a
  reference from a structural region never reaches the graph. When any edge in
  your graph is structural, `blast-radius` reports the empty result but
  withholds the safety verdict and names the files it could not resolve across.

- **Call and instantiation edges cover every Tier 1 language, at different
  depths.** A direct call records `Calls` when the name binds to a symbol the
  file can see, and `Instantiates` when the thing called turns out to be a type;
  a callee that binds to nothing records no edge rather than a guess, and
  neither does a call whose only available target is a third-party package. In
  Rust, `Foo::new(..)` names the method that runs rather than the type, and
  `Foo {..}` is construction outright; in Go, `pkg.Func(..)` is recorded because
  that is how every cross-package call is written, and `Foo{..}` is
  construction. `obj.method()` records no call edge — it is a property access on the receiver's declared type instead.
  C and C++ are the narrowest: only a bare `foo(..)` and `new Foo(..)`, because
  C++ methods are not symbols in this graph — but a bare call does cross
  translation units through a header prototype. Tier 2 languages see calls within a
  single file only. A query filtered to a kind no edge in your graph carries is
  reported as such, so an empty result is never mistaken for "nothing calls
  this".

- **A C call through a header prototype reaches the definition.** C splits a
  call across two files: the caller includes a header that declares the
  function, and the definition lives in a `.c` file the caller never sees. The
  caller attaches to the definition, so `blast-radius` on the definition finds
  it. The link is made when a header declares a name and exactly one file both
  defines that name and includes that header — an agreement between two files,
  not a name matched across the repository. Two definitions, or a definition
  whose file does not include the header, leave the call unlinked rather than
  guessed at.

- **A method call is only attributed where the receiver's type is declared.** A
  parameter or field gives the receiver a type, so `u.Greeting()` is recorded as
  a property access on that type. A local bound by inference — Go's `u := f()`,
  and the same shape elsewhere — is not tracked, so the method call reaches the
  graph as nothing at all.

- **Test detection is by file convention only.** `_test.go`, `*.test.ts`,
  `test_*.py`, a Rust crate's `tests/` — the rule each language's own tooling
  uses. A Rust `#[cfg(test)] mod tests` inside an ordinary source file is *not*
  recognised, and the symbols inside such a module are not indexed at all, so
  those tests produce no `tests` edges and `graphyn tests` will not suggest
  them. Integration tests under `tests/` are covered.

- **`graphyn tests` licenses an omission, and says when it cannot.** Naming the
  tests that cover a change is a claim that the ones left out cannot fail. Exit
  `3` means something could be missing — an uncovered changed symbol, a
  structural region, or a test selected on weak evidence — and the full suite
  should run. A test the change itself modified is reported apart from coverage
  and does not mark anything covered.

- **Three audit detectors ship; three are held back, and say why.**
  `assertion-removal` has no signal at all: assertion calls are not in the
  graph, since Rust's assert macros are unexpanded token trees and framework
  calls resolve to nothing. `scope-creep` has no denominator — Graphyn is never
  given an intended scope. `special-casing` needs control-flow analysis, and the
  graph models references between symbols rather than flow inside one. `graphyn
  audit` names every check that did not run, because an absent check otherwise
  reads as a passing one.

- **`test-tampering` fires on coverage disappearing, not on a test changing.**
  Changing a function and updating its test together is ordinary work; a
  detector firing there would be switched off in a week. It reports a test that
  *stopped* covering a symbol which changed in the same diff.

- **`graphyn context` is worth less against a text search than against reading
  files.** Orienting on `RepoIR` here costs ~420 estimated tokens against
  ~57,900 for reading those thirty files whole — but only ~670 for `rg -l`,
  which answers the same "which files touch this" question. The signature
  skeleton, the part a text search cannot produce, lands for 1 of those 31
  entries: inbound edges to a widely-used type are dominated by imports, which
  adapters attribute to the file's module symbol rather than the function using
  it. Useful for a narrow neighbourhood; thin for a widely-imported type. Token
  figures are byte-based estimates, not a tokenizer's count.

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

| Java | 2 | `.java` | symbols and intra-file references only — build with `--features java` |

### Tiers

**Tier 1 — resolved.** Imports, aliases and declared types are resolved, so
member access is attributed to the type a value was declared as. Safe to gate
CI on.

**Tier 2 — structural.** Symbols and intra-file references, extracted from the
grammar's own `tags.scm`. No cross-file import resolution, no aliases, no
declared types. Genuinely useful for locating symbols and for intra-file blast
radius — and explicitly not something to gate on. A tags query reports that a
call to `foo` happened; it does not say which `foo`, and guessing by name
across a repository is the bug Graphyn exists to avoid.

`graphyn status` reports the tier of every language your build carries, and the
share of edges that resolved — overall and per language. An enforcement tool
that tells you "91% of references in this repository resolved, and here is what
it could not" is worth more than one implying completeness.

Queries take `--min-confidence resolved` to restrict an answer to edges bound
through imports, aliases and declared types. The threshold is applied while
traversing, not to the results, so nothing is reached by way of an edge below
it. On a Tier 2 repository that correctly returns nothing.

Tier 2 today: Java, Ruby, C# — each behind its own feature, none in `default`.

Still planned as Tier 2: Kotlin, PHP, Swift, Scala, SQL, Lua, Bash. These are
not held up by Graphyn's architecture but by the grammar crates: adding one
needs a crate that both works against the `tree-sitter` version Graphyn pins
and ships its own `tags.scm`. Several of the obvious candidates currently fail
one or the other — Scala and Lua pin an incompatible `tree-sitter`, Swift and
PHP resolve to a second copy of it, and Kotlin, SQL and Bash ship no tags query
for the analyzer to run. A `tree-sitter` upgrade unblocks most of them.

## Slim builds

**A default `graphyn` carries the Tier 1 languages only.** `full` is the
everything binary, and is what the releases publish. This matters beyond size:
a default build does not merely resolve less of a polyglot repository, it does
not look at the Tier 2 files at all — which is why it can report 100% coverage
on a repository the `full` binary reports 99.7% on.

| Build | Languages | Size |
|---|---|---|
| `--no-default-features --features typescript` | TypeScript / JavaScript | 21.2 MB |
| `default` | TypeScript, Python, Rust, Go, C/C++ | 28.8 MB |
| `full` | the above plus Java, Ruby, C# | 36.6 MB |

Measured on one machine with `--release`; treat them as relative, not absolute.

```bash
cargo install graphyn-cli --no-default-features --features python
cargo install graphyn-cli --features full
```

Tier 1 features: `typescript` (includes JavaScript), `python`, `rust`, `go`,
`c` (includes C++) — these are `default`. Tier 2 features: `java`, `ruby`,
`csharp`; `full` is everything. `graphyn status` and `--help` report what your
build can analyse, and a build skips files in languages it does not carry
rather than failing on them.

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
