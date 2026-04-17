# graphyn-cli

Command-line interface for Graphyn.

`graphyn-cli` is the user-facing binary (`graphyn`) that drives analysis, querying, live watch updates, status, and MCP serving.

## Commands

- `graphyn analyze <path> [--include <csv>] [--exclude <csv>] [--no-gitignore]`
- `graphyn query blast-radius <symbol> [--file <path>] [--depth <n>]`
- `graphyn query usages <symbol> [--file <path>]`
- `graphyn query deps <symbol> [--file <path>] [--depth <n>]`
- `graphyn watch <path> [--include <csv>] [--exclude <csv>] [--no-gitignore]`
- `graphyn serve [--stdio | --port 7700]`
- `graphyn status <path>`

## Scan filtering

- `.gitignore` is respected by default for `analyze` and `watch`.
- `--include` and `--exclude` accept comma-separated multiple patterns.
- `--exclude` has higher priority than `--include`.

Example:

```bash
graphyn analyze . --include "src/**,packages/api/**" --exclude "**/*.test.ts,dist/**"
```

## Notes

- This crate orchestrates `graphyn-core`, `graphyn-adapter-ts`, `graphyn-store`, and `graphyn-mcp`.
- Query behavior is deterministic and backed by persisted graph data.
