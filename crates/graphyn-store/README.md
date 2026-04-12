# graphyn-store

Persistence and hot-cache crate for Graphyn.

`graphyn-store` persists graph snapshots into RocksDB and provides an in-memory hot query cache for fast repeated lookups.

## Responsibilities

- Save/load `GraphynGraph` snapshots (`RocksGraphStore`)
- Snapshot conversion (`GraphSnapshot`)
- In-memory cache for query results (`HotQueryCache`)

## Main APIs

- `RocksGraphStore::open(path)`
- `RocksGraphStore::save_graph(&graph)`
- `RocksGraphStore::load_graph()`
- `HotQueryCache::{new, put, get, invalidate, clear, stats}`

## Minimal usage

```rust
use std::path::Path;
use graphyn_store::RocksGraphStore;

let store = RocksGraphStore::open(Path::new(".graphyn/db"))?;
let _graph = store.load_graph()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Notes

- Snapshot encoding is binary and round-trip safe for literal backslash sequences.
- Storage concerns are isolated here; graph/query logic stays in `graphyn-core`.
