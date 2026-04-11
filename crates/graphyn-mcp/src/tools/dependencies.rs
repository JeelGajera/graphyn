//! MCP tool: get_dependencies
//!
//! Returns everything a given symbol depends on — its full dependency tree.

use schemars::JsonSchema;
use serde::Deserialize;

use graphyn_core::graph::GraphynGraph;
use graphyn_core::query;

use crate::context_builder;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DependenciesParams {
    /// The symbol name to analyze
    pub symbol: String,
    /// Optional: narrow to a specific file path if symbol name is ambiguous
    pub file: Option<String>,
    /// How many hops to traverse. Default 3. Max 10.
    pub depth: Option<i32>,
}

pub fn execute(graph: &GraphynGraph, params: DependenciesParams) -> Result<String, String> {
    let depth = params.depth.unwrap_or(3).max(1).min(10) as usize;

    let edges = query::dependencies(graph, &params.symbol, params.file.as_deref(), Some(depth))
        .map_err(|e| format!("{e}"))?;

    Ok(context_builder::format_dependencies(
        graph,
        &params.symbol,
        params.file.as_deref(),
        depth,
        &edges,
    ))
}
