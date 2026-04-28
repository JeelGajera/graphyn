//! Graphyn MCP server — exposes graph query tools to coding agents.
//!
//! Uses the official `rmcp` SDK with stdio transport. Agents like Cursor
//! and Claude Code spawn `graphyn serve --stdio` and communicate via
//! JSON-RPC over stdin/stdout.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ServerCapabilities;
use rmcp::model::ServerInfo;
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};

use graphyn_core::graph::GraphynGraph;
use graphyn_store::RocksGraphStore;

use crate::tools::{blast_radius, dependencies, refresh_graph, symbol_usages};

/// The Graphyn MCP server. Holds a loaded graph and tool router.
#[derive(Clone)]
pub struct GraphynMcpServer {
    graph: Arc<RwLock<GraphynGraph>>,
    #[allow(dead_code)]
    repo_root: PathBuf,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl GraphynMcpServer {
    /// Create a new server by loading the graph from `.graphyn/db`.
    pub fn new(repo_root: PathBuf) -> Result<Self, String> {
        let graph = load_graph(&repo_root)?;
        Ok(Self {
            graph: Arc::new(RwLock::new(graph)),
            repo_root,
            tool_router: Self::tool_router(),
        })
    }

    /// Given a symbol name, returns all symbols that depend on it and would
    /// be affected by changes. Resolves aliases. Tracks property-level access.
    #[tool(
        name = "get_blast_radius",
        description = "Given a symbol name, returns all symbols that depend on it and would be affected by changes. Resolves aliases. Tracks property-level access."
    )]
    async fn get_blast_radius(
        &self,
        params: Parameters<blast_radius::BlastRadiusParams>,
    ) -> Result<String, String> {
        let graph = self
            .graph
            .read()
            .map_err(|_| "graph lock poisoned".to_string())?;
        blast_radius::execute(&graph, params.0)
    }

    /// Returns everything a given symbol depends on — its full dependency tree.
    #[tool(
        name = "get_dependencies",
        description = "Returns everything a given symbol depends on — its full dependency tree."
    )]
    async fn get_dependencies(
        &self,
        params: Parameters<dependencies::DependenciesParams>,
    ) -> Result<String, String> {
        let graph = self
            .graph
            .read()
            .map_err(|_| "graph lock poisoned".to_string())?;
        dependencies::execute(&graph, params.0)
    }

    /// Finds all usages of a symbol across the codebase, including under
    /// aliases and re-exports.
    #[tool(
        name = "get_symbol_usages",
        description = "Finds all usages of a symbol across the codebase, including under aliases and re-exports. Use this when you need to find all references before renaming or deleting a symbol."
    )]
    async fn get_symbol_usages(
        &self,
        params: Parameters<symbol_usages::SymbolUsagesParams>,
    ) -> Result<String, String> {
        let graph = self
            .graph
            .read()
            .map_err(|_| "graph lock poisoned".to_string())?;
        symbol_usages::execute(&graph, params.0)
    }

    /// Rebuild and persist the graph index. Agents can call this after code changes.
    #[tool(
        name = "refresh_graph_index",
        description = "Re-analyzes the repository and updates the persisted graph index. Supports include/exclude filters and .gitignore respect."
    )]
    async fn refresh_graph_index(
        &self,
        params: Parameters<refresh_graph::RefreshGraphParams>,
    ) -> Result<String, String> {
        let (new_graph, result) = refresh_graph::execute(&self.repo_root, params.0)?;
        {
            let mut graph = self
                .graph
                .write()
                .map_err(|_| "graph lock poisoned".to_string())?;
            *graph = new_graph;
        }

        Ok(format!(
            "Graph index refreshed successfully.\nFiles indexed: {}\nSymbols: {}\nRelationships: {}\nAlias chains: {}\nDiagnostics: {}",
            result.files_indexed,
            result.symbols,
            result.relationships,
            result.alias_chains,
            result.diagnostics
        ))
    }
}

#[tool_handler]
impl ServerHandler for GraphynMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new(
                "graphyn",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Graphyn is a code intelligence engine. Use get_blast_radius to find \
                 what will break if you change a symbol, get_dependencies to see what \
                 a symbol depends on, get_symbol_usages to find every usage \
                 including aliased imports, and refresh_graph_index after repository changes.",
            )
    }
}

/// Start the MCP server over stdio transport.
pub async fn serve_stdio(repo_root: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let server = GraphynMcpServer::new(repo_root)?;

    let transport = rmcp::transport::io::stdio();
    let running_service = server.serve(transport).await?;

    // Wait until the client disconnects
    running_service.waiting().await?;

    Ok(())
}

// ── helpers ──────────────────────────────────────────────────

fn load_graph(repo_root: &Path) -> Result<GraphynGraph, String> {
    let db = repo_root.join(".graphyn").join("db");
    if !db.exists() {
        // Not an error — agent can call refresh_graph_index later
        eprintln!(
            "[graphyn] No graph at {}. Starting with empty graph. \
             Run `graphyn analyze` or call refresh_graph_index tool.",
            db.display()
        );
        return Ok(GraphynGraph::new());
    }
    let store = RocksGraphStore::open(&db).map_err(|e| format!("failed to open store: {e}"))?;
    store
        .load_graph()
        .map_err(|e| format!("failed to load graph: {e}"))
}
