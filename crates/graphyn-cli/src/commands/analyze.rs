use std::path::Path;
use std::time::Instant;

use graphyn_adapter_ts::analyze_files;
use graphyn_adapter_ts::language::is_supported_source_file;
use graphyn_core::graph::GraphynGraph;
use graphyn_core::ir::RepoIR;
use graphyn_core::resolver::AliasResolver;
use graphyn_core::scan::{parse_csv_patterns, walk_source_files_with_config, ScanConfig};
use graphyn_store::RocksGraphStore;

use crate::output;

pub fn run(
    path: &str,
    include_csv: Option<&str>,
    exclude_csv: Option<&str>,
    respect_gitignore: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let root =
        std::fs::canonicalize(path).map_err(|e| format!("cannot access '{}': {}", path, e))?;

    output::banner("analyze");
    output::info(&format!(
        "Analyzing {}",
        output::file_path(&root.display().to_string())
    ));
    output::blank();

    let start = Instant::now();

    // ── 1. Parse with the TypeScript adapter ─────────────────
    output::step("Scanning files", "...");
    let scan_config = ScanConfig {
        include_patterns: parse_csv_patterns(include_csv),
        exclude_patterns: parse_csv_patterns(exclude_csv),
        respect_gitignore,
    };

    let files = walk_source_files_with_config(&root, &scan_config, is_supported_source_file)
        .map_err(|e| format!("scan failed: {e}"))?;

    let repo_ir = analyze_files(&root, &files).map_err(|e| format!("analysis failed: {e}"))?;

    let file_count = repo_ir.files.len();
    let error_count: usize = repo_ir.files.iter().map(|f| f.parse_errors.len()).sum();
    output::step(
        "Parsed files",
        &format!("{file_count} OK, {error_count} error(s)"),
    );

    // ── 2. Build graph ───────────────────────────────────────
    let (graph, stats) = build_graph(&repo_ir);
    output::step(
        "Built graph",
        &format!("{} symbols, {} edges", stats.symbols, stats.relationships),
    );
    output::step(
        "Resolved aliases",
        &format!("{} chain(s)", stats.alias_chains),
    );

    // ── 3. Persist ───────────────────────────────────────────
    let db = super::db_path(&root);
    if let Some(parent) = db.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store = RocksGraphStore::open(&db).map_err(|e| format!("failed to open store: {e}"))?;
    store
        .save_graph(&graph)
        .map_err(|e| format!("failed to persist graph: {e}"))?;

    output::step(
        "Persisted to",
        &root.join(".graphyn/").display().to_string(),
    );

    // ── 4. Summary ───────────────────────────────────────────
    let elapsed = start.elapsed();
    output::section("Summary");
    output::stat_highlight("Symbols", &stats.symbols.to_string());
    output::stat_highlight("Relationships", &stats.relationships.to_string());
    output::stat_highlight("Files indexed", &file_count.to_string());
    output::stat_highlight("Alias chains", &stats.alias_chains.to_string());
    output::stat(
        "Respect .gitignore",
        if scan_config.respect_gitignore {
            "yes"
        } else {
            "no"
        },
    );
    if !scan_config.include_patterns.is_empty() {
        output::stat("Include", &scan_config.include_patterns.join(", "));
    }
    if !scan_config.exclude_patterns.is_empty() {
        output::stat("Exclude", &scan_config.exclude_patterns.join(", "));
    }

    if !repo_ir.language_stats.is_empty() {
        output::blank();
        let mut langs: Vec<_> = repo_ir.language_stats.iter().collect();
        langs.sort_by(|a, b| b.1.cmp(a.1));
        for (lang, count) in langs {
            output::stat(&format!("  {lang}"), &format!("{count} file(s)"));
        }
    }

    if error_count > 0 {
        output::blank();
        output::warning(&format!("{error_count} parse error(s) encountered"));
        for file_ir in &repo_ir.files {
            for err in &file_ir.parse_errors {
                output::dim_line(&format!("  {} — {err}", file_ir.file));
            }
        }
    }

    output::done(&format!("Analysis complete ({:.0?})", elapsed));
    Ok(())
}

// ── graph construction ───────────────────────────────────────

pub struct AnalyzeStats {
    pub symbols: usize,
    pub relationships: usize,
    pub alias_chains: usize,
}

pub fn build_graph(repo_ir: &RepoIR) -> (GraphynGraph, AnalyzeStats) {
    let mut graph = GraphynGraph::new();
    let resolver = AliasResolver::default();

    // Add all symbols
    for file_ir in &repo_ir.files {
        for symbol in &file_ir.symbols {
            graph.add_symbol(symbol.clone());
        }
    }

    // Add all relationships and populate alias chains
    for file_ir in &repo_ir.files {
        for relationship in &file_ir.relationships {
            graph.add_relationship(relationship);
        }
        resolver.ingest_relationships(&graph, &file_ir.relationships);
    }

    let stats = AnalyzeStats {
        symbols: graph.symbols.len(),
        relationships: graph.graph.edge_count(),
        alias_chains: graph.alias_chains.len(),
    };

    (graph, stats)
}

pub fn load_graph(repo_root: &Path) -> Result<GraphynGraph, Box<dyn std::error::Error>> {
    let db = super::db_path(repo_root);
    if !db.exists() {
        return Err(format!(
            "No graph found at {}. Run {} first.",
            db.display(),
            output::bold_cyan("graphyn analyze <path>"),
        )
        .into());
    }
    let store = RocksGraphStore::open(&db).map_err(|e| format!("failed to open store: {e}"))?;
    let graph = store
        .load_graph()
        .map_err(|e| format!("failed to load graph: {e}"))?;
    Ok(graph)
}
