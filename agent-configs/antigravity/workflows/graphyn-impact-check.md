# Graphyn Impact Check

Use this workflow before changing, deleting, renaming, or moving a symbol.

## Steps

1. Identify the symbol name and file path.
2. Refresh or create the graph if needed:

```bash
graphyn analyze .
```

3. Check blast radius:

```bash
graphyn query blast-radius SymbolName
```

4. Check usages before deletion or rename:

```bash
graphyn query usages SymbolName
```

5. Check dependencies before moving or extracting:

```bash
graphyn query deps SymbolName
```

6. Summarize risky dependents, aliases, and accessed properties before editing.

