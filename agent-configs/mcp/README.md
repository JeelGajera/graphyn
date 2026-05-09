# Graphyn MCP Configs

Graphyn exposes MCP tools over stdio:

```bash
graphyn serve --stdio
```

Use these snippets as starting points. Set `GRAPHYN_ROOT` to the repository you
want Graphyn to index/query.

## Cursor

Project file: `.cursor/mcp.json`

```json
{
  "mcpServers": {
    "graphyn": {
      "command": "graphyn",
      "args": ["serve", "--stdio"],
      "env": {
        "GRAPHYN_ROOT": "${workspaceFolder}"
      }
    }
  }
}
```

## Claude Code

Project file: `.mcp.json`

```json
{
  "mcpServers": {
    "graphyn": {
      "type": "stdio",
      "command": "graphyn",
      "args": ["serve", "--stdio"],
      "env": {
        "GRAPHYN_ROOT": "."
      }
    }
  }
}
```

Equivalent CLI:

```bash
claude mcp add-json graphyn '{"type":"stdio","command":"graphyn","args":["serve","--stdio"],"env":{"GRAPHYN_ROOT":"."}}' --scope project
```

### Antigravity

Antigravity reads MCP servers from `.gemini/settings.json`:

```json
{
  "mcpServers": {
    "graphyn": {
      "command": "graphyn",
      "args": ["serve", "--stdio"],
      "cwd": "${workspaceFolder}"
    }
  }
}
```

## Gemini CLI

Project or user `settings.json` entry:

```json
{
  "mcpServers": {
    "graphyn": {
      "command": "graphyn",
      "args": ["serve", "--stdio"],
      "env": {
        "GRAPHYN_ROOT": "."
      }
    }
  }
}
```

## OpenAI Codex CLI

User file: `~/.codex/config.toml`

```toml
[mcp_servers.graphyn]
command = "graphyn"
args = ["serve", "--stdio"]

[mcp_servers.graphyn.env]
GRAPHYN_ROOT = "."
```

## Common Checks

```bash
graphyn analyze .
graphyn status
graphyn serve --stdio
```

If a client cannot find `graphyn`, use the full path from:

```bash
which graphyn
```

