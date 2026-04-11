use graphyn_core::ir::SymbolKind;

use crate::output;

pub fn run(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::fs::canonicalize(path)
        .map_err(|e| format!("cannot access '{}': {}", path, e))?;
    let graph = super::analyze::load_graph(&root)?;

    output::banner("status");
    output::info(&format!(
        "Graph: {}",
        output::file_path(&root.join(".graphyn/db").display().to_string()),
    ));
    output::blank();

    // ── core stats ───────────────────────────────────────────
    output::section("Graph Overview");
    output::stat_highlight("Symbols", &graph.symbols.len().to_string());
    output::stat_highlight("Relationships", &graph.graph.edge_count().to_string());
    output::stat_highlight(
        "Files indexed",
        &graph.file_index.len().to_string(),
    );
    output::stat_highlight(
        "Alias chains",
        &graph.alias_chains.len().to_string(),
    );

    // ── symbol breakdown ─────────────────────────────────────
    let mut kind_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for entry in graph.symbols.iter() {
        *kind_counts
            .entry(format_kind(&entry.value().kind).to_string())
            .or_insert(0) += 1;
    }

    if !kind_counts.is_empty() {
        output::section("Symbol Kinds");
        for (kind, count) in &kind_counts {
            output::stat(&format!("  {kind}"), &count.to_string());
        }
    }

    // ── files by symbol count ────────────────────────────────
    let mut file_counts: Vec<(String, usize)> = graph
        .file_index
        .iter()
        .map(|entry| (entry.key().clone(), entry.value().len()))
        .collect();
    file_counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    if !file_counts.is_empty() {
        output::section("Files by Symbol Count");
        for (file, count) in file_counts.iter().take(10) {
            output::stat(
                &format!("  {}", output::file_path(file)),
                &format!("{count} symbol(s)"),
            );
        }
        if file_counts.len() > 10 {
            output::dim_line(&format!(
                "  … and {} more file(s)",
                file_counts.len() - 10
            ));
        }
    }

    // ── alias chains detail ──────────────────────────────────
    if !graph.alias_chains.is_empty() {
        output::section("Alias Chains");
        for entry in graph.alias_chains.iter() {
            let canonical_id = entry.key();
            let canonical_name = graph
                .symbols
                .get(canonical_id.as_str())
                .map(|s| s.name.clone())
                .unwrap_or_else(|| canonical_id.clone());

            let aliases: Vec<String> = entry
                .value()
                .iter()
                .map(|a| output::alias_tag(&a.alias_name))
                .collect();
            println!(
                "  {} → {}",
                output::symbol_name(&canonical_name),
                aliases.join(", "),
            );
        }
    }

    output::blank();
    Ok(())
}

fn format_kind(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::TypeAlias => "type",
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Property => "property",
        SymbolKind::Variable => "variable",
        SymbolKind::Module => "module",
        SymbolKind::Enum => "enum",
        SymbolKind::EnumVariant => "variant",
    }
}
