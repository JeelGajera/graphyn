# graphyn-adapter-rust

Rust adapter for Graphyn based on tree-sitter.

`graphyn-adapter-rust` parses `.rs` sources, extracts symbols and relationships into Graphyn IR, and resolves `use` paths through the crate's module tree.

## Responsibilities

- Parses with `tree-sitter-rust`
- Builds the module tree from `mod` declarations, `lib.rs`/`main.rs`/`mod.rs` layout
- Extracts IR from AST:
  - symbols (structs, enums, traits, impls, functions, type aliases, consts)
  - `use` groups, aliases (`use a::B as C`) and glob imports
  - property-access relationships keyed by the receiver's declared type
- Resolves use-paths through the module that declares the name, following `pub use` chains
- Records `#[derive(...)]` traits and recovers field access from macro token trees

## Main entrypoint

```rust
use std::path::{Path, PathBuf};
use graphyn_adapter_rust::analyze_files;

let files = vec![PathBuf::from("src/models/user.rs")];
let repo_ir = analyze_files(Path::new("./my-repo"), &files)?;
println!("files parsed: {}", repo_ir.files.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Callers normally reach this crate through [`graphyn-adapter-dispatch`](../graphyn-adapter-dispatch), which routes files to the adapter that owns their language.

## Test fixtures

Validated against committed fixtures under `fixtures/adapter-rust/`, including the alias-import bug scenario, module trees, use groups, trait impls and derive macros.

## Notes

- Structural analysis is deterministic — no LLM is involved.
- Macro bodies are token trees. Field access inside `format!` and friends is recovered by scanning tokens; macro-generated code is not expanded.
- Parse, resolution and skip diagnostics are recorded in `FileIR.parse_errors`.
