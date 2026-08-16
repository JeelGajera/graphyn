# graphyn-adapter-python

Python adapter for Graphyn based on tree-sitter.

`graphyn-adapter-python` parses `.py` and `.pyi` sources, extracts symbols and relationships into Graphyn IR, and resolves Python's import forms to canonical symbol IDs.

## Responsibilities

- Parses with `tree-sitter-python`
- Extracts IR from AST:
  - symbols (classes, functions, assignments, type aliases)
  - imports, aliases and `__init__.py` re-export chains
  - property-access relationships keyed by the receiver's declared type
- Resolves relative imports (`from . import x`, `from ..pkg import y`), star imports and `TYPE_CHECKING` guards
- Classifies Pydantic, Django and dataclass models so their fields are attributed to the model

## Main entrypoint

```rust
use std::path::{Path, PathBuf};
use graphyn_adapter_python::analyze_files;

let files = vec![PathBuf::from("src/models/user.py")];
let repo_ir = analyze_files(Path::new("./my-repo"), &files)?;
println!("files parsed: {}", repo_ir.files.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Callers normally reach this crate through [`graphyn-adapter-dispatch`](../graphyn-adapter-dispatch), which routes files to the adapter that owns their language.

## Test fixtures

Validated against committed fixtures under `fixtures/adapter-py/`, including the alias-import bug scenario, relative imports, star imports and framework model detection.

## Notes

- Structural analysis is deterministic — no LLM is involved.
- Member access is attributed to the type a value was declared as, so `payload.user_id` is recorded against `UserPayload` however the variable was named.
- Parse, resolution and skip diagnostics are recorded in `FileIR.parse_errors`.
