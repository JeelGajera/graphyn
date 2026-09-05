//! MCP tool: get_dependencies
//!
//! Returns everything a given symbol depends on — its full dependency tree.

use schemars::JsonSchema;
use serde::Deserialize;

use graphyn_core::graph::GraphynGraph;
use graphyn_core::query;

use crate::context_builder;
use crate::tools::kinds::{absent_kinds_warning, mask_from_names, KINDS_DOC};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DependenciesParams {
    /// The symbol name to analyze
    pub symbol: String,
    /// Optional: narrow to a specific file path if symbol name is ambiguous
    pub file: Option<String>,
    /// How many hops to traverse. Default 3. Max 10.
    pub depth: Option<i32>,
    #[schemars(description = KINDS_DOC)]
    pub kinds: Option<Vec<String>>,
}

pub fn execute(graph: &GraphynGraph, params: DependenciesParams) -> Result<String, String> {
    let depth = params.depth.unwrap_or(3).clamp(1, 10) as usize;

    let mask = mask_from_names(&params.kinds)?;
    let edges = query::dependencies(
        graph,
        &params.symbol,
        params.file.as_deref(),
        Some(depth),
        mask,
    )
    .map_err(|e| format!("{e}"))?;

    let body = context_builder::format_dependencies(
        graph,
        &params.symbol,
        params.file.as_deref(),
        depth,
        &edges,
    );
    Ok(match absent_kinds_warning(&mask, graph) {
        Some(warning) => format!("{warning}\n\n{body}"),
        None => body,
    })
}
