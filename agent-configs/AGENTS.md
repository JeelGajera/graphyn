# Graphyn Agent Instructions

Use Graphyn as the source of truth for symbol impact analysis in this repository.
Graphyn supports TypeScript/JavaScript, Python, Rust, Go, C, and C++ analysis.

## Before Risky Code Changes

Run a Graphyn query before changing, deleting, renaming, or moving:

- exported classes, functions, interfaces, types, DTOs, schemas, services, repositories, controllers, mappers, or shared utilities
- constructor-injected dependencies or DI provider/module wiring
- fields or properties on shared payload/model types

Prefer MCP tools when available:

- `get_blast_radius(symbol)` before changing a symbol
- `get_symbol_usages(symbol)` before deleting or renaming
- `get_dependencies(symbol)` before moving or extracting code
- `refresh_graph(path=".")` after generating or moving files

Fallback CLI commands:

```bash
graphyn analyze .
graphyn query blast-radius SymbolName
graphyn query usages SymbolName
graphyn query deps SymbolName
```

## How To Interpret Results

- Treat aliased imports as high-risk usages.
- Check `properties_accessed` before changing fields on a model or DTO.
- If a symbol is ambiguous, rerun with `--file`.
- If no graph exists, run `graphyn analyze .` before continuing.
- If include filters are used, prefer recursive globs like `src/**/*.ts`.
- Graphyn respects `.gitignore` by default. If a file is ignored, use
  `graphyn analyze . --no-gitignore` or call `refresh_graph` with
  `respect_gitignore: false`.

## Do Not

- Do not rely only on text search for impact analysis.
- Do not delete or rename public symbols without checking usages.
- Do not ignore Graphyn warnings about unresolved imports unless you can explain
  why the target is external or intentionally unavailable.
