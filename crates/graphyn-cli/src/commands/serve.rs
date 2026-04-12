use crate::output;

pub fn run(port: u16, stdio: bool) -> Result<(), Box<dyn std::error::Error>> {
    if stdio {
        // stdio transport — the primary MCP interface
        let repo_root = std::env::var("GRAPHYN_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| ".".into()));

        let repo_root = std::fs::canonicalize(&repo_root)
            .map_err(|e| format!("cannot access '{}': {}", repo_root.display(), e))?;

        // Build the tokio runtime and start the MCP server.
        // We write status to stderr so it doesn't interfere with the
        // JSON-RPC protocol on stdout.
        eprintln!(
            "[graphyn] Starting MCP server (stdio) for {}",
            repo_root.display()
        );

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("failed to start async runtime: {e}"))?;

        rt.block_on(async { graphyn_mcp::server::serve_stdio(repo_root).await })?;

        Ok(())
    } else {
        // TCP transport — not yet supported
        output::banner("serve");
        output::info(&format!("Transport: TCP on port {port}"));
        output::blank();
        output::warning("TCP transport is not yet implemented.");
        output::dim_line("Use --stdio for agent integration (Cursor, Claude Code).");
        output::blank();
        output::dim_line("  graphyn serve --stdio");
        output::blank();
        Ok(())
    }
}
