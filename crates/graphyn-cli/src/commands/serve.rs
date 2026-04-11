use crate::output;

pub fn run(port: u16, stdio: bool) -> Result<(), Box<dyn std::error::Error>> {
    output::banner("serve");

    if stdio {
        output::info("Transport: stdio");
    } else {
        output::info(&format!("Transport: TCP on port {port}"));
    }

    output::blank();
    output::warning("MCP server is not yet implemented.");
    output::dim_line("This will be available in the next release (Step 5).");
    output::blank();
    output::dim_line("Once ready, agents will connect via:");
    output::blank();

    if stdio {
        output::dim_line("  # Cursor — .cursor/mcp.json");
        output::dim_line("  {");
        output::dim_line("    \"mcpServers\": {");
        output::dim_line("      \"graphyn\": {");
        output::dim_line("        \"command\": \"graphyn\",");
        output::dim_line("        \"args\": [\"serve\", \"--stdio\"]");
        output::dim_line("      }");
        output::dim_line("    }");
        output::dim_line("  }");
    } else {
        output::dim_line(&format!("  graphyn serve --port {port}"));
    }
    output::blank();

    Ok(())
}
