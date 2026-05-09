# Introduction

> Understand the blast radius before you pull the trigger.

Graphyn is a code intelligence engine that models your codebase as a living graph of symbol relationships. It gives coding agents a precise knowledge of what will break before making a change or how a change will affect the codebase.

It is not a search tool. It is not a chatbot over your repo. It is a deterministic relationship graph that resolves aliases, tracks property-level access, and answers the questions your agent needs answered before touching anything.

## Core Features

- **Blast radius analysis**: Know exactly which files and symbols are affected by a change.
- **Alias resolution**: Tracks imports like `import { A as B }` across your entire project.
- **Property-level tracking**: Understands not just that a class is used, but which specific properties are accessed.
- **MCP Integration**: Speaks standard Model Context Protocol, making it compatible with Cursor, Claude Code, and other modern agents.
- **High Performance**: Sub-second queries and fast incremental updates.

## Getting Started

To install Graphyn, follow the instructions in the [README](https://github.com/JeelGajera/graphyn#install). Once installed, you can start analyzing your repository:

```bash
graphyn analyze ./my-repo
```
