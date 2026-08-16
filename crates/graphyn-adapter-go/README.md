# graphyn-adapter-go

Go adapter for Graphyn based on tree-sitter.

`graphyn-adapter-go` parses `.go` sources, extracts symbols and relationships into Graphyn IR, and resolves package imports against the `go.mod` files found in the repository.

## Responsibilities

- Parses with `tree-sitter-go`
- Discovers modules per directory from `go.mod`, so modules in subdirectories resolve
- Extracts IR from AST:
  - symbols (structs, interfaces, functions, methods, type aliases, consts)
  - package imports, including named and dot aliases
  - property-access relationships keyed by the receiver's declared type
- Points imports at a synthetic package symbol, with qualified references carrying the specific type
- Detects structural interface satisfaction by comparing method sets from the symbol table

## Main entrypoint

```rust
use std::path::{Path, PathBuf};
use graphyn_adapter_go::analyze_files;

let files = vec![PathBuf::from("models/user.go")];
let repo_ir = analyze_files(Path::new("./my-repo"), &files)?;
println!("files parsed: {}", repo_ir.files.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Callers normally reach this crate through [`graphyn-adapter-dispatch`](../graphyn-adapter-dispatch), which routes files to the adapter that owns their language.

## Test fixtures

Validated against committed fixtures under `fixtures/adapter-go/`, including the alias-import bug scenario, multi-file packages, struct embedding and interface implementation.

## Notes

- Structural analysis is deterministic — no LLM is involved.
- Interface matching is per-package. A type satisfying an interface declared in another package is not reported, because matching every method set against every interface repository-wide produces far more noise than signal.
- Parse, resolution and skip diagnostics are recorded in `FileIR.parse_errors`.
