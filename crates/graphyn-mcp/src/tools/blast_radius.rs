//! MCP tool: get_blast_radius
//!
//! Given a symbol name, returns all symbols that depend on it and would be
//! affected by changes. Resolves aliases. Tracks property-level access.

use schemars::JsonSchema;
use serde::Deserialize;

use graphyn_core::graph::GraphynGraph;
use graphyn_core::query;

use crate::context_builder;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlastRadiusParams {
    /// The symbol name to analyze (e.g. 'UserPayload', 'authService', 'processOrder')
    pub symbol: String,
    /// Optional: narrow to a specific file path if symbol name is ambiguous
    pub file: Option<String>,
    /// How many hops to traverse. Default 3. Max 10.
    pub depth: Option<i32>,
}

pub fn execute(graph: &GraphynGraph, params: BlastRadiusParams) -> Result<String, String> {
    let depth = params.depth.unwrap_or(3).max(1).min(10) as usize;

    let edges = query::blast_radius(graph, &params.symbol, params.file.as_deref(), Some(depth))
        .map_err(|e| format!("{e}"))?;

    Ok(context_builder::format_blast_radius(
        graph,
        &params.symbol,
        params.file.as_deref(),
        depth,
        &edges,
    ))
}
