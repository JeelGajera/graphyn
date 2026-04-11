use std::fmt::{Display, Formatter};
use std::path::Path;

use graphyn_core::graph::GraphynGraph;
use graphyn_core::ir::{Language, Relationship, RelationshipKind, Symbol, SymbolKind};
use graphyn_core::resolver::{AliasEntry, AliasScope};
use rocksdb::{Options, DB};

const KEY_GRAPH_SNAPSHOT: &[u8] = b"graph_snapshot_v1";

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
        let bytes = snapshot.to_bytes();
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

    fn to_bytes(&self) -> Vec<u8> {
        let mut out = String::new();

        out.push_str("[SYMBOLS]\n");
        for symbol in &self.symbols {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                esc(&symbol.id),
                esc(&symbol.name),
                symbol_kind_to_str(&symbol.kind),
                language_to_str(&symbol.language),
                esc(&symbol.file),
                symbol.line_start,
                symbol.line_end,
                esc(symbol.signature.as_deref().unwrap_or(""))
            ));
        }

        out.push_str("[RELATIONSHIPS]\n");
        for relationship in &self.relationships {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                esc(&relationship.from),
                esc(&relationship.to),
                relationship_kind_to_str(&relationship.kind),
                esc(relationship.alias.as_deref().unwrap_or("")),
                esc(&relationship.properties_accessed.join(",")),
                esc(&relationship.context),
                esc(&relationship.file),
                relationship.line,
            ));
        }

        out.push_str("[ALIASES]\n");
        for (canonical, entries) in &self.alias_chains {
            for entry in entries {
                out.push_str(&format!(
                    "{}\t{}\t{}\t{}\n",
                    esc(canonical),
                    esc(&entry.alias_name),
                    esc(&entry.defined_in_file),
                    alias_scope_to_str(&entry.scope)
                ));
            }
        }

        out.into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let content = String::from_utf8(bytes.to_vec())
            .map_err(|err| StoreError::Serialization(format!("invalid utf8 snapshot: {err}")))?;

        let mut section = "";
        let mut symbols = Vec::new();
        let mut relationships = Vec::new();
        let mut aliases_map: std::collections::BTreeMap<String, Vec<AliasEntry>> =
            std::collections::BTreeMap::new();

        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = line;
                continue;
            }

            let cols: Vec<&str> = line.split('\t').collect();
            match section {
                "[SYMBOLS]" => {
                    if cols.len() != 8 {
                        return Err(StoreError::Serialization(
                            "invalid symbol row format".to_string(),
                        ));
                    }

                    symbols.push(Symbol {
                        id: unesc(cols[0]),
                        name: unesc(cols[1]),
                        kind: str_to_symbol_kind(cols[2])?,
                        language: str_to_language(cols[3])?,
                        file: unesc(cols[4]),
                        line_start: cols[5]
                            .parse::<u32>()
                            .map_err(|err| StoreError::Serialization(err.to_string()))?,
                        line_end: cols[6]
                            .parse::<u32>()
                            .map_err(|err| StoreError::Serialization(err.to_string()))?,
                        signature: {
                            let sig = unesc(cols[7]);
                            if sig.is_empty() {
                                None
                            } else {
                                Some(sig)
                            }
                        },
                    });
                }
                "[RELATIONSHIPS]" => {
                    if cols.len() != 8 {
                        return Err(StoreError::Serialization(
                            "invalid relationship row format".to_string(),
                        ));
                    }

                    let properties_csv = unesc(cols[4]);
                    let properties_accessed = if properties_csv.is_empty() {
                        Vec::new()
                    } else {
                        properties_csv.split(',').map(|s| s.to_string()).collect()
                    };

                    relationships.push(Relationship {
                        from: unesc(cols[0]),
                        to: unesc(cols[1]),
                        kind: str_to_relationship_kind(cols[2])?,
                        alias: {
                            let alias = unesc(cols[3]);
                            if alias.is_empty() {
                                None
                            } else {
                                Some(alias)
                            }
                        },
                        properties_accessed,
                        context: unesc(cols[5]),
                        file: unesc(cols[6]),
                        line: cols[7]
                            .parse::<u32>()
                            .map_err(|err| StoreError::Serialization(err.to_string()))?,
                    });
                }
                "[ALIASES]" => {
                    if cols.len() != 4 {
                        return Err(StoreError::Serialization(
                            "invalid alias row format".to_string(),
                        ));
                    }

                    let canonical = unesc(cols[0]);
                    aliases_map.entry(canonical).or_default().push(AliasEntry {
                        alias_name: unesc(cols[1]),
                        defined_in_file: unesc(cols[2]),
                        scope: str_to_alias_scope(cols[3])?,
                    });
                }
                _ => {}
            }
        }

        let alias_chains = aliases_map.into_iter().collect();

        Ok(Self {
            symbols,
            relationships,
            alias_chains,
        })
    }
}

fn esc(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unesc(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek().copied() {
                Some('t') => {
                    let _ = chars.next();
                    out.push('\t');
                }
                Some('n') => {
                    let _ = chars.next();
                    out.push('\n');
                }
                Some('\\') => {
                    let _ = chars.next();
                    out.push('\\');
                }
                Some(other) => {
                    out.push(other);
                    let _ = chars.next();
                }
                None => out.push(ch),
            }
        } else {
            out.push(ch);
        }
    }

    out
}

fn symbol_kind_to_str(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "Class",
        SymbolKind::Interface => "Interface",
        SymbolKind::TypeAlias => "TypeAlias",
        SymbolKind::Function => "Function",
        SymbolKind::Method => "Method",
        SymbolKind::Property => "Property",
        SymbolKind::Variable => "Variable",
        SymbolKind::Module => "Module",
        SymbolKind::Enum => "Enum",
        SymbolKind::EnumVariant => "EnumVariant",
    }
}

fn str_to_symbol_kind(input: &str) -> Result<SymbolKind, StoreError> {
    match input {
        "Class" => Ok(SymbolKind::Class),
        "Interface" => Ok(SymbolKind::Interface),
        "TypeAlias" => Ok(SymbolKind::TypeAlias),
        "Function" => Ok(SymbolKind::Function),
        "Method" => Ok(SymbolKind::Method),
        "Property" => Ok(SymbolKind::Property),
        "Variable" => Ok(SymbolKind::Variable),
        "Module" => Ok(SymbolKind::Module),
        "Enum" => Ok(SymbolKind::Enum),
        "EnumVariant" => Ok(SymbolKind::EnumVariant),
        other => Err(StoreError::Serialization(format!(
            "unknown symbol kind: {other}"
        ))),
    }
}

fn language_to_str(language: &Language) -> &'static str {
    match language {
        Language::TypeScript => "TypeScript",
        Language::JavaScript => "JavaScript",
        Language::Python => "Python",
        Language::Rust => "Rust",
        Language::Go => "Go",
        Language::Java => "Java",
    }
}

fn str_to_language(input: &str) -> Result<Language, StoreError> {
    match input {
        "TypeScript" => Ok(Language::TypeScript),
        "JavaScript" => Ok(Language::JavaScript),
        "Python" => Ok(Language::Python),
        "Rust" => Ok(Language::Rust),
        "Go" => Ok(Language::Go),
        "Java" => Ok(Language::Java),
        other => Err(StoreError::Serialization(format!(
            "unknown language: {other}"
        ))),
    }
}

fn relationship_kind_to_str(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Imports => "Imports",
        RelationshipKind::Calls => "Calls",
        RelationshipKind::Extends => "Extends",
        RelationshipKind::Implements => "Implements",
        RelationshipKind::UsesType => "UsesType",
        RelationshipKind::AccessesProperty => "AccessesProperty",
        RelationshipKind::ReExports => "ReExports",
        RelationshipKind::Instantiates => "Instantiates",
    }
}

fn str_to_relationship_kind(input: &str) -> Result<RelationshipKind, StoreError> {
    match input {
        "Imports" => Ok(RelationshipKind::Imports),
        "Calls" => Ok(RelationshipKind::Calls),
        "Extends" => Ok(RelationshipKind::Extends),
        "Implements" => Ok(RelationshipKind::Implements),
        "UsesType" => Ok(RelationshipKind::UsesType),
        "AccessesProperty" => Ok(RelationshipKind::AccessesProperty),
        "ReExports" => Ok(RelationshipKind::ReExports),
        "Instantiates" => Ok(RelationshipKind::Instantiates),
        other => Err(StoreError::Serialization(format!(
            "unknown relationship kind: {other}"
        ))),
    }
}

fn alias_scope_to_str(scope: &AliasScope) -> &'static str {
    match scope {
        AliasScope::ImportAlias => "ImportAlias",
        AliasScope::ReExport => "ReExport",
        AliasScope::BarrelReExport => "BarrelReExport",
        AliasScope::DefaultImport => "DefaultImport",
    }
}

fn str_to_alias_scope(input: &str) -> Result<AliasScope, StoreError> {
    match input {
        "ImportAlias" => Ok(AliasScope::ImportAlias),
        "ReExport" => Ok(AliasScope::ReExport),
        "BarrelReExport" => Ok(AliasScope::BarrelReExport),
        "DefaultImport" => Ok(AliasScope::DefaultImport),
        other => Err(StoreError::Serialization(format!(
            "unknown alias scope: {other}"
        ))),
    }
}
