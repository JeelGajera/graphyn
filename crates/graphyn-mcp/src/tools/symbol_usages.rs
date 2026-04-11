//! MCP tool: get_symbol_usages
//!
//! Finds all usages of a symbol across the codebase, including under aliases
//! and re-exports. Use this when you need to find all references before
//! renaming or deleting a symbol.

use schemars::JsonSchema;
use serde::Deserialize;

use graphyn_core::graph::GraphynGraph;
use graphyn_core::query;

use crate::context_builder;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SymbolUsagesParams {
    /// The symbol name to find
    pub symbol: String,
    /// Optional: narrow to a specific file path if symbol name is ambiguous
    pub file: Option<String>,
    /// Include usages under aliased imports. Default true. Never set false.
    pub include_aliases: Option<bool>,
}

pub fn execute(graph: &GraphynGraph, params: SymbolUsagesParams) -> Result<String, String> {
    let include_aliases = params.include_aliases.unwrap_or(true);

    let edges = query::symbol_usages(
        graph,
        &params.symbol,
        params.file.as_deref(),
        include_aliases,
    )
    .map_err(|e| format!("{e}"))?;

    Ok(context_builder::format_symbol_usages(
        graph,
        &params.symbol,
        params.file.as_deref(),
        &edges,
    ))
}
