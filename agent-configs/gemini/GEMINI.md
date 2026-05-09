# Graphyn Guidance

Use Graphyn before risky code changes.

## When To Query

- Changing exported classes, functions, interfaces, types, DTOs, schemas, or services
- Deleting or renaming symbols
- Moving modules
- Editing shared model fields or DI wiring

## Preferred Tools

Use MCP if available:

- `get_blast_radius`
- `get_symbol_usages`
- `get_dependencies`
- `refresh_graph`

Fallback:

```bash
graphyn analyze .
graphyn query blast-radius SymbolName
graphyn query usages SymbolName
graphyn query deps SymbolName
```

Pay attention to aliases and `properties_accessed`.

