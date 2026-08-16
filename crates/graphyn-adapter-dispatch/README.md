# graphyn-adapter-dispatch

Language dispatch layer for Graphyn.

`graphyn-adapter-dispatch` is the single entrypoint the CLI and MCP server call to turn a list of source files into `RepoIR`. It detects each file's language, groups files by the adapter that owns them, runs those groups in parallel, and merges the results.

## Responsibilities

- Maps file extensions to languages via `graphyn_core::scan::detect_language_from_extension`
- Groups files by owning adapter — TypeScript and JavaScript share one, C and C++ share one
- Runs each group in parallel with rayon and merges the `FileIR`s into a single `RepoIR`
- Skips files in languages this build does not support yet
- Reports which languages the build can analyse, for help text and diagnostics

## Determinism

Grouping uses an ordered key rather than `HashMap` iteration order, and each group's files are sorted before analysis. `RepoIR.files` determines the order symbols and edges are inserted into the graph, and Graphyn's first documented guarantee is that the graph is deterministic — hash-ordered grouping made two runs over identical input produce differently-ordered output.

## Main entrypoint

```rust
use std::path::{Path, PathBuf};
use graphyn_adapter_dispatch::{analyze_files, supported_languages};

let files = vec![
    PathBuf::from("src/app.ts"),
    PathBuf::from("service/main.py"),
];
let repo_ir = analyze_files(Path::new("./my-repo"), &files)?;
println!("languages: {:?}", supported_languages());
println!("files parsed: {}", repo_ir.files.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Limits

Imports resolve within one language. Each adapter resolves its own group independently, so a Python module importing a TypeScript file through a build step is not linked.

## Notes

- Adapter failures surface as `DispatchError`, tagged with the language whose adapter failed.
- No parsing or graph logic lives here; it belongs to the adapters and `graphyn-core`.
