# graphyn-adapter-c

C and C++ adapter for Graphyn based on tree-sitter.

`graphyn-adapter-c` parses C and C++ translation units and headers, extracts symbols and relationships into Graphyn IR, and resolves `#include` directives to the header's module symbol.

## Responsibilities

- Parses with `tree-sitter-c` and `tree-sitter-cpp`, chosen per file extension
  (`.c` `.h` → C; `.cpp` `.cc` `.cxx` `.hpp` `.hxx` `.hh` → C++)
- Extracts IR from AST:
  - symbols (structs, unions, enums, classes, functions, typedefs)
  - `#include` edges, `typedef` aliases and C++ `using` aliases
  - base classes and namespace-qualified names
  - property-access relationships keyed by the receiver's declared type
- Records a struct or class only where it is defined, so a `struct Foo *p` parameter does not mint a second symbol

## Main entrypoint

```rust
use std::path::{Path, PathBuf};
use graphyn_adapter_c::analyze_files;

let files = vec![PathBuf::from("src/mapper.c")];
let repo_ir = analyze_files(Path::new("./my-repo"), &files)?;
println!("files parsed: {}", repo_ir.files.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Callers normally reach this crate through [`graphyn-adapter-dispatch`](../graphyn-adapter-dispatch), which routes files to the adapter that owns their language.

## Test fixtures

Validated against committed fixtures under `fixtures/adapter-c/`, including the alias-import bug scenario, include resolution, typedefs, C++ inheritance and namespace `using`.

## Notes

- Structural analysis is deterministic — no LLM is involved.
- The preprocessor is not run. `#include` is resolved structurally against files in the repository; conditional compilation is not evaluated.
- Templates are parsed but not instantiated. `vector<Foo>` records a reference to `Foo`; it does not model what the instantiation generates.
- Parse, resolution and skip diagnostics are recorded in `FileIR.parse_errors`.
