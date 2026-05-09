# Graphyn Instructions For GitHub Copilot

This repository uses Graphyn for deterministic code relationship analysis.

Before proposing changes that affect exported symbols, public types, DTO fields,
services, repositories, controllers, mappers, or shared utilities:

- Ask for or run `graphyn query blast-radius <SymbolName>`.
- Use `graphyn query usages <SymbolName>` before deleting or renaming.
- Use `graphyn query deps <SymbolName>` before moving or extracting code.

Graphyn resolves aliased imports and reports property-level access. Do not rely
only on text search when reasoning about impact.

If the graph is stale or missing, run:

```bash
graphyn analyze .
```

