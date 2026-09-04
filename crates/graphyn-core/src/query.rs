use std::collections::{BTreeSet, HashSet, VecDeque};

use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::error::GraphynError;
use crate::graph::GraphynGraph;
use crate::index::find_symbol_id;
use crate::ir::{RelationshipKind, SymbolId};

const DEFAULT_DEPTH: usize = 3;
const MAX_DEPTH: usize = 10;

/// Every kind of relationship, in a fixed order.
///
/// The order is the enum's own declaration order and is what `--kind help` and
/// any listing render, so it must not depend on how a `match` happens to be
/// written.
pub const ALL_KINDS: [RelationshipKind; 8] = [
    RelationshipKind::Imports,
    RelationshipKind::Calls,
    RelationshipKind::Extends,
    RelationshipKind::Implements,
    RelationshipKind::UsesType,
    RelationshipKind::AccessesProperty,
    RelationshipKind::ReExports,
    RelationshipKind::Instantiates,
];

/// Kinds nothing currently emits.
///
/// A filter that can only ever match nothing is a trap in a tool meant to
/// gate changes: a rule scoped to such a kind would never fire and would read
/// as a pass. Naming them here lets the CLI say so rather than silently
/// returning an empty result.
///
/// `Calls` left this list when structural (Tier 2) analysis began emitting it
/// from a grammar's own tags query. No Tier 1 language emits it yet, so in a
/// default build a `calls` filter still matches nothing — which is why this
/// list is a stopgap. The honest version of this check asks the graph in hand
/// which kinds it actually contains, rather than consulting a constant that
/// has to be maintained by hand; that belongs with the confidence model, whose
/// whole subject is how much a given graph knows.
pub const UNEMITTED_KINDS: [RelationshipKind; 1] = [RelationshipKind::Instantiates];

/// The name a kind is known by on the command line and in JSON.
pub fn kind_name(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Imports => "imports",
        RelationshipKind::Calls => "calls",
        RelationshipKind::Extends => "extends",
        RelationshipKind::Implements => "implements",
        RelationshipKind::UsesType => "uses-type",
        RelationshipKind::AccessesProperty => "accesses-property",
        RelationshipKind::ReExports => "re-exports",
        RelationshipKind::Instantiates => "instantiates",
    }
}

/// Parse a kind from its command-line name.
pub fn parse_kind(name: &str) -> Option<RelationshipKind> {
    ALL_KINDS
        .iter()
        .find(|k| kind_name(k) == name)
        .map(|k| (*k).clone())
}

/// Which relationship kinds a traversal should follow.
///
/// `RelationshipMeta` has always carried the kind of every edge, and
/// `traverse` has always discarded it, so a query could not tell a test's
/// reference from an import from a trait implementation. Every gate and
/// finding in the later phases needs that distinction, and the data was
/// already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationshipKindMask(u16);

impl Default for RelationshipKindMask {
    fn default() -> Self {
        Self::all()
    }
}

impl RelationshipKindMask {
    /// Follow every kind — what traversal did before it could filter.
    pub fn all() -> Self {
        Self(u16::MAX)
    }

    pub fn none() -> Self {
        Self(0)
    }

    pub fn from_kinds(kinds: &[RelationshipKind]) -> Self {
        kinds
            .iter()
            .fold(Self::none(), |mask, kind| mask.with(kind.clone()))
    }

    pub fn with(self, kind: RelationshipKind) -> Self {
        Self(self.0 | (1 << Self::bit(&kind)))
    }

    pub fn contains(&self, kind: &RelationshipKind) -> bool {
        self.0 & (1 << Self::bit(kind)) != 0
    }

    /// True when the mask admits everything, so callers can skip reporting a
    /// filter the user did not ask for.
    pub fn is_all(&self) -> bool {
        ALL_KINDS.iter().all(|k| self.contains(k))
    }

    pub fn is_empty(&self) -> bool {
        ALL_KINDS.iter().all(|k| !self.contains(k))
    }

    /// The kinds in this mask, in [`ALL_KINDS`] order.
    pub fn kinds(&self) -> Vec<RelationshipKind> {
        ALL_KINDS
            .iter()
            .filter(|k| self.contains(k))
            .cloned()
            .collect()
    }

    fn bit(kind: &RelationshipKind) -> u16 {
        match kind {
            RelationshipKind::Imports => 0,
            RelationshipKind::Calls => 1,
            RelationshipKind::Extends => 2,
            RelationshipKind::Implements => 3,
            RelationshipKind::UsesType => 4,
            RelationshipKind::AccessesProperty => 5,
            RelationshipKind::ReExports => 6,
            RelationshipKind::Instantiates => 7,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryEdge {
    pub from: SymbolId,
    pub to: SymbolId,
    /// What kind of reference this edge records.
    ///
    /// Carried on `RelationshipMeta` since 0.2.0 and dropped at the query
    /// boundary until now, which left every consumer treating a property
    /// access and a trait implementation as the same fact.
    pub kind: RelationshipKind,
    pub file: String,
    pub line: u32,
    pub alias: Option<String>,
    pub properties_accessed: Vec<String>,
    pub context: String,
    pub hop: usize,
}

/// Split edges into those that reach the symbol under its own name and those
/// that rename it.
///
/// The distinction drives the "HIGH RISK" labelling in both the CLI and the MCP
/// server: a caller that imports `UserPayload as ResponseModel` will not turn up
/// in a text search for the original name, which is exactly the case Graphyn
/// exists to surface.
///
/// An `alias` equal to the symbol's own name is not a rename. Adapters record
/// the local name on type-reference edges whether or not it differs, so testing
/// `alias.is_some()` alone flagged ordinary same-name references as high risk
/// and buried the genuine renames among them.
pub fn partition_by_alias<'e>(
    graph: &GraphynGraph,
    edges: &'e [QueryEdge],
) -> (Vec<&'e QueryEdge>, Vec<&'e QueryEdge>) {
    let mut direct = Vec::new();
    let mut aliased = Vec::new();

    for edge in edges {
        if is_renamed(graph, edge) {
            aliased.push(edge);
        } else {
            direct.push(edge);
        }
    }

    (direct, aliased)
}

/// True when `edge` refers to its target under a name other than the target's.
pub fn is_renamed(graph: &GraphynGraph, edge: &QueryEdge) -> bool {
    let Some(alias) = edge.alias.as_deref() else {
        return false;
    };
    match graph.symbols.get(&edge.to) {
        // A qualified reference such as `models.UserPayload` names the symbol
        // directly; only the final segment is compared.
        Some(symbol) => alias.rsplit(['.', ':']).next().unwrap_or(alias) != symbol.name,
        // Unknown target: an alias is the only name we have, so treat it as one.
        None => true,
    }
}

/// True when every reference that touches `property` reaches its type under a
/// different name.
///
/// Such a property is invisible to a text search for the declaring type, which
/// is the case worth flagging. One that is also read under the type's own name
/// somewhere is not.
///
/// Two things this got wrong before. It tested `alias.is_some()`, the same
/// mistake `partition_by_alias` was fixed for — adapters record the local name
/// on a reference whether or not it differs, so an ordinary same-name access
/// was labelled aliased. And `all` over an empty iterator is true, so a
/// property no edge actually touches was labelled aliased as well.
pub fn is_aliased_only_property(graph: &GraphynGraph, edges: &[QueryEdge], property: &str) -> bool {
    let mut touching = edges
        .iter()
        .filter(|e| e.properties_accessed.iter().any(|p| p == property))
        .peekable();

    if touching.peek().is_none() {
        return false;
    }
    touching.all(|e| is_renamed(graph, e))
}

pub fn blast_radius(
    graph: &GraphynGraph,
    symbol: &str,
    file: Option<&str>,
    depth: Option<usize>,
    kinds: RelationshipKindMask,
) -> Result<Vec<QueryEdge>, GraphynError> {
    let effective_depth = depth.unwrap_or(DEFAULT_DEPTH);
    if effective_depth > MAX_DEPTH {
        return Err(GraphynError::InvalidDepth {
            depth: effective_depth,
            max: MAX_DEPTH,
        });
    }

    let root = find_symbol_id(graph, symbol, file)?;
    traverse(graph, &root, effective_depth, Direction::Incoming, kinds)
}

pub fn dependencies(
    graph: &GraphynGraph,
    symbol: &str,
    file: Option<&str>,
    depth: Option<usize>,
    kinds: RelationshipKindMask,
) -> Result<Vec<QueryEdge>, GraphynError> {
    let effective_depth = depth.unwrap_or(DEFAULT_DEPTH);
    if effective_depth > MAX_DEPTH {
        return Err(GraphynError::InvalidDepth {
            depth: effective_depth,
            max: MAX_DEPTH,
        });
    }

    let root = find_symbol_id(graph, symbol, file)?;
    traverse(graph, &root, effective_depth, Direction::Outgoing, kinds)
}

pub fn symbol_usages(
    graph: &GraphynGraph,
    symbol: &str,
    file: Option<&str>,
    include_aliases: bool,
    kinds: RelationshipKindMask,
) -> Result<Vec<QueryEdge>, GraphynError> {
    let root = find_symbol_id(graph, symbol, file)?;
    let mut results = traverse(graph, &root, 1, Direction::Incoming, kinds)?;

    if include_aliases {
        if let Some(aliases) = graph.alias_chains.get(&root) {
            let alias_set: HashSet<String> = aliases.iter().map(|a| a.alias_name.clone()).collect();
            for edge in &mut results {
                if edge.alias.is_none() && edge.context.contains(" as ") {
                    let alias = edge
                        .context
                        .split(" as ")
                        .nth(1)
                        .and_then(|v| v.split_whitespace().next())
                        .map(|s| s.trim_matches(|c: char| c == ',' || c == ';').to_string());
                    if let Some(found) = alias {
                        if alias_set.contains(&found) {
                            edge.alias = Some(found);
                        }
                    }
                }
            }
        }
    } else {
        results.retain(|edge| edge.alias.is_none());
    }

    dedupe_edges(results)
}

/// Walk outward from `root`, following only edges whose kind is in `kinds`.
///
/// Filtering happens on the edge itself rather than on the collected result,
/// so an excluded kind also stops the walk continuing through it. Reaching a
/// symbol only via a kind the caller excluded means it is not reachable under
/// that question: asking "what imports this?" should not return something that
/// merely inherits from an importer.
fn traverse(
    graph: &GraphynGraph,
    root: &SymbolId,
    max_depth: usize,
    direction: Direction,
    kinds: RelationshipKindMask,
) -> Result<Vec<QueryEdge>, GraphynError> {
    let Some(root_node) = graph.node_index.get(root).map(|v| *v) else {
        return Err(GraphynError::SymbolNotFound(root.clone()));
    };

    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut results = Vec::new();

    queue.push_back((root_node, 0usize));
    visited.insert(root_node);

    while let Some((node, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        for edge in graph.graph.edges_directed(node, direction) {
            if !kinds.contains(&edge.weight().kind) {
                continue;
            }
            let neighbor = if direction == Direction::Incoming {
                edge.source()
            } else {
                edge.target()
            };

            let from_id = graph
                .graph
                .node_weight(edge.source())
                .cloned()
                .ok_or_else(|| GraphynError::GraphCorrupt("Missing source node".to_string()))?;
            let to_id = graph
                .graph
                .node_weight(edge.target())
                .cloned()
                .ok_or_else(|| GraphynError::GraphCorrupt("Missing target node".to_string()))?;
            let meta = edge.weight();

            results.push(QueryEdge {
                from: from_id,
                to: to_id,
                kind: meta.kind.clone(),
                file: meta.file.clone(),
                line: meta.line,
                alias: meta.alias.clone(),
                properties_accessed: meta.properties_accessed.clone(),
                context: meta.context.clone(),
                hop: depth + 1,
            });

            if visited.insert(neighbor) {
                queue.push_back((neighbor, depth + 1));
            }
        }
    }

    dedupe_edges(results)
}

/// Collapse rows that describe the same fact, then order them.
///
/// One reference produces several edges when the source location is
/// attributed at more than one level — a class-level edge and a method-level
/// edge to the same target at the same line — and the old key included
/// `from`, so all of them survived as separate rows. A 64-file project
/// reported 196 "dependents" for 38 referencing files, and the aliased
/// findings that are the whole point of the tool sat below 160 rows of
/// duplicates.
///
/// The identity of a reference is what a reader can act on: which symbol,
/// where, under what name, by what kind of reference. `from` is not part of
/// that — the location already says where — so it no longer splits rows.
/// `kind` is, because an `extends` and a field read at one line are two
/// different facts about the code.
///
/// Sorting happens before collapsing rather than after, so which row survives
/// is decided by the ordering rather than by traversal order. The lowest hop
/// wins, which is the shortest path to the symbol and the one worth showing.
fn dedupe_edges(mut edges: Vec<QueryEdge>) -> Result<Vec<QueryEdge>, GraphynError> {
    edges.sort_by(|a, b| {
        a.hop
            .cmp(&b.hop)
            .then(a.file.cmp(&b.file))
            .then(a.line.cmp(&b.line))
            .then(a.to.cmp(&b.to))
            .then(kind_name(&a.kind).cmp(kind_name(&b.kind)))
            .then(a.alias.cmp(&b.alias))
            .then(a.from.cmp(&b.from))
    });

    let mut seen = BTreeSet::new();
    edges.retain(|edge| {
        seen.insert((
            edge.to.clone(),
            edge.file.clone(),
            edge.line,
            edge.alias.clone(),
            kind_name(&edge.kind),
        ))
    });

    Ok(edges)
}
