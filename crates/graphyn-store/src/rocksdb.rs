use std::fmt::{Display, Formatter};
use std::path::Path;

use graphyn_core::graph::GraphynGraph;
use graphyn_core::ir::{Language, Relationship, RelationshipKind, Symbol, SymbolKind};
use graphyn_core::resolver::{AliasEntry, AliasScope};
use rocksdb::{Options, DB};

const KEY_GRAPH_SNAPSHOT: &[u8] = b"graph_snapshot_v1";
const SNAPSHOT_VERSION: u8 = 1;

#[derive(Debug)]
pub enum StoreError {
    RocksDb(String),
    Serialization(String),
    SnapshotNotFound,
}

impl Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RocksDb(err) => write!(f, "rocksdb error: {err}"),
            Self::Serialization(err) => write!(f, "serialization error: {err}"),
            Self::SnapshotNotFound => write!(f, "snapshot not found"),
        }
    }
}

impl std::error::Error for StoreError {}

#[derive(Debug, Clone)]
pub struct GraphSnapshot {
    pub symbols: Vec<Symbol>,
    pub relationships: Vec<Relationship>,
    pub alias_chains: Vec<(String, Vec<AliasEntry>)>,
}

pub struct RocksGraphStore {
    db: DB,
}

impl RocksGraphStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let mut options = Options::default();
        options.create_if_missing(true);
        let db = DB::open(&options, path).map_err(|err| StoreError::RocksDb(err.to_string()))?;
        Ok(Self { db })
    }

    pub fn save_graph(&self, graph: &GraphynGraph) -> Result<(), StoreError> {
        let snapshot = GraphSnapshot::from_graph(graph)?;
        self.save_snapshot(&snapshot)
    }

    pub fn load_graph(&self) -> Result<GraphynGraph, StoreError> {
        let snapshot = self.load_snapshot()?;
        snapshot.into_graph()
    }

    pub fn save_snapshot(&self, snapshot: &GraphSnapshot) -> Result<(), StoreError> {
        let bytes = snapshot.to_bytes()?;
        self.db
            .put(KEY_GRAPH_SNAPSHOT, bytes)
            .map_err(|err| StoreError::RocksDb(err.to_string()))
    }

    pub fn load_snapshot(&self) -> Result<GraphSnapshot, StoreError> {
        let bytes = self
            .db
            .get(KEY_GRAPH_SNAPSHOT)
            .map_err(|err| StoreError::RocksDb(err.to_string()))?
            .ok_or(StoreError::SnapshotNotFound)?;

        GraphSnapshot::from_bytes(&bytes)
    }
}

impl GraphSnapshot {
    pub fn from_graph(graph: &GraphynGraph) -> Result<Self, StoreError> {
        let mut symbols: Vec<Symbol> = graph
            .symbols
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        symbols.sort_by(|a, b| a.id.cmp(&b.id));

        let mut relationships = Vec::new();
        for edge_id in graph.graph.edge_indices() {
            let (source_idx, target_idx) = graph
                .graph
                .edge_endpoints(edge_id)
                .ok_or_else(|| StoreError::Serialization("missing edge endpoints".to_string()))?;
            let from = graph
                .graph
                .node_weight(source_idx)
                .cloned()
                .ok_or_else(|| StoreError::Serialization("missing source node".to_string()))?;
            let to = graph
                .graph
                .node_weight(target_idx)
                .cloned()
                .ok_or_else(|| StoreError::Serialization("missing target node".to_string()))?;
            let meta = graph
                .graph
                .edge_weight(edge_id)
                .ok_or_else(|| StoreError::Serialization("missing edge metadata".to_string()))?;

            relationships.push(Relationship {
                from,
                to,
                kind: meta.kind.clone(),
                alias: meta.alias.clone(),
                properties_accessed: meta.properties_accessed.clone(),
                context: meta.context.clone(),
                file: meta.file.clone(),
                line: meta.line,
            });
        }
        relationships.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.line.cmp(&b.line))
                .then(a.from.cmp(&b.from))
                .then(a.to.cmp(&b.to))
        });

        let mut alias_chains: Vec<(String, Vec<AliasEntry>)> = graph
            .alias_chains
            .iter()
            .map(|entry| {
                let mut aliases = entry.value().clone();
                aliases.sort_by(|a, b| {
                    a.defined_in_file
                        .cmp(&b.defined_in_file)
                        .then(a.alias_name.cmp(&b.alias_name))
                });
                (entry.key().clone(), aliases)
            })
            .collect();
        alias_chains.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(Self {
            symbols,
            relationships,
            alias_chains,
        })
    }

    pub fn into_graph(self) -> Result<GraphynGraph, StoreError> {
        let mut graph = GraphynGraph::new();

        for symbol in self.symbols {
            graph.add_symbol(symbol);
        }

        for relationship in &self.relationships {
            graph.add_relationship(relationship);
        }

        for (canonical_id, aliases) in self.alias_chains {
            graph.alias_chains.insert(canonical_id, aliases);
        }

        Ok(graph)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        let mut out = Vec::new();

        write_u8(&mut out, SNAPSHOT_VERSION);

        write_u32(&mut out, self.symbols.len() as u32);
        for symbol in &self.symbols {
            write_string(&mut out, &symbol.id)?;
            write_string(&mut out, &symbol.name)?;
            write_u8(&mut out, symbol_kind_to_u8(&symbol.kind));
            write_u8(&mut out, language_to_u8(&symbol.language));
            write_string(&mut out, &symbol.file)?;
            write_u32(&mut out, symbol.line_start);
            write_u32(&mut out, symbol.line_end);
            write_optional_string(&mut out, symbol.signature.as_deref())?;
        }

        write_u32(&mut out, self.relationships.len() as u32);
        for relationship in &self.relationships {
            write_string(&mut out, &relationship.from)?;
            write_string(&mut out, &relationship.to)?;
            write_u8(&mut out, relationship_kind_to_u8(&relationship.kind));
            write_optional_string(&mut out, relationship.alias.as_deref())?;
            write_u32(&mut out, relationship.properties_accessed.len() as u32);
            for prop in &relationship.properties_accessed {
                write_string(&mut out, prop)?;
            }
            write_string(&mut out, &relationship.context)?;
            write_string(&mut out, &relationship.file)?;
            write_u32(&mut out, relationship.line);
        }

        write_u32(&mut out, self.alias_chains.len() as u32);
        for (canonical, entries) in &self.alias_chains {
            write_string(&mut out, canonical)?;
            write_u32(&mut out, entries.len() as u32);
            for entry in entries {
                write_string(&mut out, &entry.alias_name)?;
                write_string(&mut out, &entry.defined_in_file)?;
                write_u8(&mut out, alias_scope_to_u8(&entry.scope));
            }
        }

        Ok(out)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let mut cursor = ByteCursor::new(bytes);

        let version = cursor.read_u8()?;
        if version != SNAPSHOT_VERSION {
            return Err(StoreError::Serialization(format!(
                "unsupported snapshot version: {version}"
            )));
        }

        let symbol_count = cursor.read_u32()? as usize;
        let mut symbols = Vec::with_capacity(symbol_count);
        for _ in 0..symbol_count {
            symbols.push(Symbol {
                id: cursor.read_string()?,
                name: cursor.read_string()?,
                kind: u8_to_symbol_kind(cursor.read_u8()?)?,
                language: u8_to_language(cursor.read_u8()?)?,
                file: cursor.read_string()?,
                line_start: cursor.read_u32()?,
                line_end: cursor.read_u32()?,
                signature: cursor.read_optional_string()?,
            });
        }

        let rel_count = cursor.read_u32()? as usize;
        let mut relationships = Vec::with_capacity(rel_count);
        for _ in 0..rel_count {
            let from = cursor.read_string()?;
            let to = cursor.read_string()?;
            let kind = u8_to_relationship_kind(cursor.read_u8()?)?;
            let alias = cursor.read_optional_string()?;
            let prop_count = cursor.read_u32()? as usize;
            let mut properties_accessed = Vec::with_capacity(prop_count);
            for _ in 0..prop_count {
                properties_accessed.push(cursor.read_string()?);
            }
            let context = cursor.read_string()?;
            let file = cursor.read_string()?;
            let line = cursor.read_u32()?;

            relationships.push(Relationship {
                from,
                to,
                kind,
                alias,
                properties_accessed,
                context,
                file,
                line,
            });
        }

        let alias_chain_count = cursor.read_u32()? as usize;
        let mut alias_chains = Vec::with_capacity(alias_chain_count);
        for _ in 0..alias_chain_count {
            let canonical = cursor.read_string()?;
            let entry_count = cursor.read_u32()? as usize;
            let mut entries = Vec::with_capacity(entry_count);
            for _ in 0..entry_count {
                entries.push(AliasEntry {
                    alias_name: cursor.read_string()?,
                    defined_in_file: cursor.read_string()?,
                    scope: u8_to_alias_scope(cursor.read_u8()?)?,
                });
            }
            alias_chains.push((canonical, entries));
        }

        if !cursor.is_at_end() {
            return Err(StoreError::Serialization(
                "trailing bytes found in snapshot".to_string(),
            ));
        }

        Ok(Self {
            symbols,
            relationships,
            alias_chains,
        })
    }
}

struct ByteCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn read_u8(&mut self) -> Result<u8, StoreError> {
        if self.pos >= self.bytes.len() {
            return Err(StoreError::Serialization(
                "unexpected EOF reading u8".to_string(),
            ));
        }
        let v = self.bytes[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_u32(&mut self) -> Result<u32, StoreError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(StoreError::Serialization(
                "unexpected EOF reading u32".to_string(),
            ));
        }
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&self.bytes[self.pos..self.pos + 4]);
        self.pos += 4;
        Ok(u32::from_le_bytes(arr))
    }

    fn read_string(&mut self) -> Result<String, StoreError> {
        let len = self.read_u32()? as usize;
        if self.pos + len > self.bytes.len() {
            return Err(StoreError::Serialization(
                "unexpected EOF reading string".to_string(),
            ));
        }
        let slice = &self.bytes[self.pos..self.pos + len];
        self.pos += len;
        String::from_utf8(slice.to_vec())
            .map_err(|err| StoreError::Serialization(format!("invalid UTF-8 string: {err}")))
    }

    fn read_optional_string(&mut self) -> Result<Option<String>, StoreError> {
        let has = self.read_u8()?;
        if has == 0 {
            Ok(None)
        } else {
            Ok(Some(self.read_string()?))
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos == self.bytes.len()
    }
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), StoreError> {
    let bytes = value.as_bytes();
    let len = u32::try_from(bytes.len())
        .map_err(|_| StoreError::Serialization("string too large".to_string()))?;
    write_u32(out, len);
    out.extend_from_slice(bytes);
    Ok(())
}

fn write_optional_string(out: &mut Vec<u8>, value: Option<&str>) -> Result<(), StoreError> {
    match value {
        Some(value) => {
            write_u8(out, 1);
            write_string(out, value)
        }
        None => {
            write_u8(out, 0);
            Ok(())
        }
    }
}

fn symbol_kind_to_u8(kind: &SymbolKind) -> u8 {
    match kind {
        SymbolKind::Class => 1,
        SymbolKind::Interface => 2,
        SymbolKind::TypeAlias => 3,
        SymbolKind::Function => 4,
        SymbolKind::Method => 5,
        SymbolKind::Property => 6,
        SymbolKind::Variable => 7,
        SymbolKind::Module => 8,
        SymbolKind::Enum => 9,
        SymbolKind::EnumVariant => 10,
    }
}

fn u8_to_symbol_kind(input: u8) -> Result<SymbolKind, StoreError> {
    match input {
        1 => Ok(SymbolKind::Class),
        2 => Ok(SymbolKind::Interface),
        3 => Ok(SymbolKind::TypeAlias),
        4 => Ok(SymbolKind::Function),
        5 => Ok(SymbolKind::Method),
        6 => Ok(SymbolKind::Property),
        7 => Ok(SymbolKind::Variable),
        8 => Ok(SymbolKind::Module),
        9 => Ok(SymbolKind::Enum),
        10 => Ok(SymbolKind::EnumVariant),
        other => Err(StoreError::Serialization(format!(
            "unknown symbol kind code: {other}"
        ))),
    }
}

fn language_to_u8(language: &Language) -> u8 {
    match language {
        Language::TypeScript => 1,
        Language::JavaScript => 2,
        Language::Python => 3,
        Language::Rust => 4,
        Language::Go => 5,
        Language::Java => 6,
    }
}

fn u8_to_language(input: u8) -> Result<Language, StoreError> {
    match input {
        1 => Ok(Language::TypeScript),
        2 => Ok(Language::JavaScript),
        3 => Ok(Language::Python),
        4 => Ok(Language::Rust),
        5 => Ok(Language::Go),
        6 => Ok(Language::Java),
        other => Err(StoreError::Serialization(format!(
            "unknown language code: {other}"
        ))),
    }
}

fn relationship_kind_to_u8(kind: &RelationshipKind) -> u8 {
    match kind {
        RelationshipKind::Imports => 1,
        RelationshipKind::Calls => 2,
        RelationshipKind::Extends => 3,
        RelationshipKind::Implements => 4,
        RelationshipKind::UsesType => 5,
        RelationshipKind::AccessesProperty => 6,
        RelationshipKind::ReExports => 7,
        RelationshipKind::Instantiates => 8,
    }
}

fn u8_to_relationship_kind(input: u8) -> Result<RelationshipKind, StoreError> {
    match input {
        1 => Ok(RelationshipKind::Imports),
        2 => Ok(RelationshipKind::Calls),
        3 => Ok(RelationshipKind::Extends),
        4 => Ok(RelationshipKind::Implements),
        5 => Ok(RelationshipKind::UsesType),
        6 => Ok(RelationshipKind::AccessesProperty),
        7 => Ok(RelationshipKind::ReExports),
        8 => Ok(RelationshipKind::Instantiates),
        other => Err(StoreError::Serialization(format!(
            "unknown relationship kind code: {other}"
        ))),
    }
}

fn alias_scope_to_u8(scope: &AliasScope) -> u8 {
    match scope {
        AliasScope::ImportAlias => 1,
        AliasScope::ReExport => 2,
        AliasScope::BarrelReExport => 3,
        AliasScope::DefaultImport => 4,
    }
}

fn u8_to_alias_scope(input: u8) -> Result<AliasScope, StoreError> {
    match input {
        1 => Ok(AliasScope::ImportAlias),
        2 => Ok(AliasScope::ReExport),
        3 => Ok(AliasScope::BarrelReExport),
        4 => Ok(AliasScope::DefaultImport),
        other => Err(StoreError::Serialization(format!(
            "unknown alias scope code: {other}"
        ))),
    }
}
