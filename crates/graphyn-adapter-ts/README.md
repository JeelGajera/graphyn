# graphyn-adapter-ts

TypeScript/JavaScript adapter for Graphyn based on tree-sitter.

`graphyn-adapter-ts` scans repository files, parses TS/TSX/JS/JSX source, extracts symbols/relationships into Graphyn IR, and resolves import/re-export links for repository-level analysis.

## Responsibilities

- Walks repo files (`.ts`, `.tsx`, `.js`, `.jsx`)
- Parses with tree-sitter dialect-aware grammars
- Extracts IR from AST:
  - symbols
  - imports/re-exports
  - property-access relationships
- Resolves unresolved import placeholders to canonical symbol IDs

## Main entrypoint

```rust
use std::path::Path;
use graphyn_adapter_ts::analyze_repo;

let repo_ir = analyze_repo(Path::new("./my-repo"))?;
println!("files parsed: {}", repo_ir.files.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Test fixtures

This crate is validated against committed fixtures under `fixtures/`, including the alias-import bug scenario.

## Notes

- Structural analysis is deterministic.
- Any unresolved bindings are recorded as parse/resolution errors in `FileIR.parse_errors`.
