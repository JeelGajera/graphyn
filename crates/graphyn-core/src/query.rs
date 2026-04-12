use std::collections::{BTreeSet, HashSet, VecDeque};

use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::error::GraphynError;
use crate::graph::GraphynGraph;
use crate::index::find_symbol_id;
use crate::ir::SymbolId;

const DEFAULT_DEPTH: usize = 3;
const MAX_DEPTH: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryEdge {
    pub from: SymbolId,
    pub to: SymbolId,
    pub file: String,
    pub line: u32,
    pub alias: Option<String>,
    pub properties_accessed: Vec<String>,
    pub context: String,
    pub hop: usize,
}

pub fn blast_radius(
    graph: &GraphynGraph,
    symbol: &str,
    file: Option<&str>,
    depth: Option<usize>,
) -> Result<Vec<QueryEdge>, GraphynError> {
    let effective_depth = depth.unwrap_or(DEFAULT_DEPTH);
    if effective_depth > MAX_DEPTH {
        return Err(GraphynError::InvalidDepth {
            depth: effective_depth,
            max: MAX_DEPTH,
        });
    }

    let root = find_symbol_id(graph, symbol, file)?;
    traverse(graph, &root, effective_depth, Direction::Incoming)
}

pub fn dependencies(
    graph: &GraphynGraph,
    symbol: &str,
    file: Option<&str>,
    depth: Option<usize>,
) -> Result<Vec<QueryEdge>, GraphynError> {
    let effective_depth = depth.unwrap_or(DEFAULT_DEPTH);
    if effective_depth > MAX_DEPTH {
        return Err(GraphynError::InvalidDepth {
            depth: effective_depth,
            max: MAX_DEPTH,
        });
    }

    let root = find_symbol_id(graph, symbol, file)?;
    traverse(graph, &root, effective_depth, Direction::Outgoing)
}

pub fn symbol_usages(
    graph: &GraphynGraph,
    symbol: &str,
    file: Option<&str>,
    include_aliases: bool,
) -> Result<Vec<QueryEdge>, GraphynError> {
    let root = find_symbol_id(graph, symbol, file)?;
    let mut results = traverse(graph, &root, 1, Direction::Incoming)?;

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

fn traverse(
    graph: &GraphynGraph,
    root: &SymbolId,
    max_depth: usize,
    direction: Direction,
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

fn dedupe_edges(mut edges: Vec<QueryEdge>) -> Result<Vec<QueryEdge>, GraphynError> {
    let mut seen = BTreeSet::new();
    edges.retain(|edge| {
        seen.insert((
            edge.from.clone(),
            edge.to.clone(),
            edge.file.clone(),
            edge.line,
            edge.alias.clone(),
        ))
    });

    edges.sort_by(|a, b| {
        a.hop
            .cmp(&b.hop)
            .then(a.file.cmp(&b.file))
            .then(a.line.cmp(&b.line))
            .then(a.from.cmp(&b.from))
            .then(a.to.cmp(&b.to))
    });

    Ok(edges)
}
