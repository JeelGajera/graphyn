# Graphyn Guidance For Claude Code

Use Graphyn before edits that can affect other files.

## Default Workflow

1. If the graph may be stale, run `graphyn analyze .` or call `refresh_graph`.
2. Before modifying a symbol, inspect `get_blast_radius`.
3. Before deleting or renaming a symbol, inspect `get_symbol_usages`.
4. Before moving code, inspect `get_dependencies`.
5. After large file moves or generated code, refresh the graph.

## Important Signals

- Aliased imports mean the same symbol may appear under another name.
- `properties_accessed` tells which fields or methods callers actually use.
- DI-heavy TypeScript projects may show constructor-injected service usage.
- Decorator/module references can create dependencies even without direct calls.

Prefer Graphyn over `rg` when the question is about relationships. Use `rg` for
plain text lookup only after Graphyn has answered the symbol graph question.

