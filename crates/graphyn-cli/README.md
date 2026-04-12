# graphyn-cli

Command-line interface for Graphyn.

`graphyn-cli` is the user-facing binary (`graphyn`) that drives analysis, querying, live watch updates, status, and MCP serving.

## Commands

- `graphyn analyze <path>`
- `graphyn query blast-radius <symbol> [--file <path>] [--depth <n>]`
- `graphyn query usages <symbol> [--file <path>]`
- `graphyn query deps <symbol> [--file <path>] [--depth <n>]`
- `graphyn watch <path>`
- `graphyn serve [--stdio | --port 7700]`
- `graphyn status <path>`

## Example flow

```bash
graphyn analyze ./my-repo
graphyn query blast-radius UserPayload --depth 3
graphyn serve --stdio
```

## Notes

- This crate orchestrates `graphyn-core`, `graphyn-adapter-ts`, `graphyn-store`, and `graphyn-mcp`.
- Query behavior is deterministic and backed by persisted graph data.
