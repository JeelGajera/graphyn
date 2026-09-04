use std::collections::BTreeMap;

use graphyn_core::graph::GraphynGraph;
use graphyn_core::ir::SymbolKind;
use graphyn_core::query::{self, QueryEdge, RelationshipKindMask};

use crate::output;

/// Resolve the `--kind` values a user passed into a traversal mask.
///
/// An unrecognised name is rejected rather than ignored. Quietly dropping it
/// would answer a narrower question than the one asked while presenting the
/// result as the answer to theirs.
pub fn mask_from_args(kinds: &[String]) -> Result<RelationshipKindMask, String> {
    if kinds.is_empty() {
        return Ok(RelationshipKindMask::all());
    }
    let mut mask = RelationshipKindMask::none();
    for name in kinds {
        let kind = query::parse_kind(name).ok_or_else(|| {
            let known: Vec<&str> = query::ALL_KINDS.iter().map(query::kind_name).collect();
            format!(
                "unknown --kind '{name}'\n  known kinds: {}",
                known.join(", ")
            )
        })?;
        mask = mask.with(kind);
    }
    Ok(mask)
}

/// Warn when a filter can only ever match nothing.
///
/// `Calls` and `Instantiates` are declared on `RelationshipKind` but no
/// adapter emits them. Returning a silent empty result for `--kind calls`
/// would read as "nothing calls this", which is the one answer Graphyn must
/// never give without warrant.
fn warn_about_unemitted(mask: &RelationshipKindMask) {
    if mask.is_all() {
        return;
    }
    let requested = mask.kinds();
    let dead: Vec<&str> = requested
        .iter()
        .filter(|k| query::UNEMITTED_KINDS.contains(k))
        .map(query::kind_name)
        .collect();
    if dead.is_empty() {
        return;
    }

    if dead.len() == requested.len() {
        output::warning(&format!(
            "every kind you asked for ({}) is unimplemented",
            dead.join(", ")
        ));
        output::dim_line("  No adapter emits it, so this result is empty regardless of your code.");
        output::dim_line("  Do not read it as evidence that nothing depends on the symbol.");
    } else {
        output::warning(&format!(
            "{} is unimplemented and contributed nothing",
            dead.join(", ")
        ));
    }
    output::blank();
}

/// Show the active filter, so a short result is never mistaken for a small
/// blast radius.
fn report_filter(mask: &RelationshipKindMask) {
    if mask.is_all() {
        return;
    }
    let names: Vec<&str> = mask.kinds().iter().map(query::kind_name).collect();
    output::stat("Kinds", &names.join(", "));
}

// ── blast-radius ─────────────────────────────────────────────

pub fn run_blast_radius(
    symbol: &str,
    file: Option<&str>,
    depth: usize,
    path: &str,
    kinds: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mask = mask_from_args(kinds)?;
    let root = super::normalize_path(
        &std::fs::canonicalize(path).map_err(|e| format!("cannot access '{}': {}", path, e))?,
    );
    let graph = super::analyze::load_graph(&root)?;

    output::banner("blast-radius");

    // look up the canonical symbol to display metadata
    let edges = query::blast_radius(&graph, symbol, file, Some(depth), mask)
        .map_err(|e| format_query_error(e, symbol))?;

    print_symbol_header(&graph, symbol, file);
    output::stat("Depth", &depth.to_string());
    report_filter(&mask);
    output::blank();
    warn_about_unemitted(&mask);

    if edges.is_empty() {
        // Never claim safety on a filtered query: the caller narrowed the
        // question, and an empty answer to a narrow question says nothing
        // about the rest of the graph.
        if mask.is_all() {
            output::success("No dependents found — safe to modify.");
        } else {
            output::success("No dependents found under the selected kinds.");
            output::dim_line("  Other relationship kinds were not searched.");
        }
        output::blank();
        return Ok(());
    }

    let (direct, aliased) = graphyn_core::query::partition_by_alias(&graph, &edges);
    let total = edges.len();
    output::section(&format!("{total} dependent(s) found"));

    if !direct.is_empty() {
        println!("  {}", output::bold(&format!("DIRECT ({})", direct.len())));
        output::blank();
        for (i, edge) in direct.iter().enumerate() {
            print_edge(i + 1, edge, &graph);
        }
    }

    if !aliased.is_empty() {
        println!(
            "  {} {}",
            output::bold_yellow(&format!("ALIASED ({})", aliased.len())),
            output::yellow("— HIGH RISK: different name in source")
        );
        output::blank();
        let offset = direct.len();
        for (i, edge) in aliased.iter().enumerate() {
            print_edge(offset + i + 1, edge, &graph);
        }
    }

    // property summary
    let props = collect_property_summary(&edges);
    if !props.is_empty() {
        output::section("Properties at Risk");
        for (prop, count) in &props {
            let aliased_note = if is_aliased_only_property(&edges, prop) {
                output::dim(" (aliased only)")
            } else {
                String::new()
            };
            println!(
                "  {}  {:<4} reference(s){}",
                output::property_name(prop),
                count,
                aliased_note,
            );
        }
        output::blank();
    }

    Ok(())
}

// ── usages ───────────────────────────────────────────────────

pub fn run_usages(
    symbol: &str,
    file: Option<&str>,
    path: &str,
    kinds: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mask = mask_from_args(kinds)?;
    let root = super::normalize_path(
        &std::fs::canonicalize(path).map_err(|e| format!("cannot access '{}': {}", path, e))?,
    );
    let graph = super::analyze::load_graph(&root)?;

    output::banner("usages");
    print_symbol_header(&graph, symbol, file);
    report_filter(&mask);
    output::blank();
    warn_about_unemitted(&mask);

    let edges = query::symbol_usages(&graph, symbol, file, true, mask)
        .map_err(|e| format_query_error(e, symbol))?;

    if edges.is_empty() {
        if mask.is_all() {
            output::success("No usages found.");
        } else {
            output::success("No usages found under the selected kinds.");
            output::dim_line("  Other relationship kinds were not searched.");
        }
        output::blank();
        return Ok(());
    }

    output::section(&format!("{} usage(s) found", edges.len()));
    for (i, edge) in edges.iter().enumerate() {
        print_edge(i + 1, edge, &graph);
    }

    Ok(())
}

// ── deps ─────────────────────────────────────────────────────

pub fn run_deps(
    symbol: &str,
    file: Option<&str>,
    depth: usize,
    path: &str,
    kinds: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mask = mask_from_args(kinds)?;
    let root = super::normalize_path(
        &std::fs::canonicalize(path).map_err(|e| format!("cannot access '{}': {}", path, e))?,
    );
    let graph = super::analyze::load_graph(&root)?;

    output::banner("dependencies");
    print_symbol_header(&graph, symbol, file);
    output::stat("Depth", &depth.to_string());
    report_filter(&mask);
    output::blank();
    warn_about_unemitted(&mask);

    let edges = query::dependencies(&graph, symbol, file, Some(depth), mask)
        .map_err(|e| format_query_error(e, symbol))?;

    if edges.is_empty() {
        if mask.is_all() {
            output::success("No dependencies found — this symbol is self-contained.");
        } else {
            output::success("No dependencies found under the selected kinds.");
            output::dim_line("  Other relationship kinds were not searched.");
        }
        output::blank();
        return Ok(());
    }

    let noun = if edges.len() == 1 {
        "dependency"
    } else {
        "dependencies"
    };
    output::section(&format!("{} {} found", edges.len(), noun));
    for (i, edge) in edges.iter().enumerate() {
        print_dep_edge(i + 1, edge, &graph);
    }

    Ok(())
}

// ── formatting helpers ───────────────────────────────────────

fn print_symbol_header(graph: &GraphynGraph, symbol: &str, file: Option<&str>) {
    // try to find the canonical symbol for metadata
    if let Some(ids) = graph.name_index.get(symbol) {
        let target_id = if let Some(file) = file {
            ids.iter().find(|id| {
                graph
                    .symbols
                    .get(*id)
                    .map(|s| s.file == file)
                    .unwrap_or(false)
            })
        } else {
            ids.first()
        };

        if let Some(id) = target_id {
            if let Some(sym) = graph.symbols.get(id) {
                output::stat_highlight(
                    "Symbol",
                    &format!(
                        "{} {}",
                        output::symbol_name(&sym.name),
                        output::dim(&format!("[{}]", format_kind(&sym.kind))),
                    ),
                );
                output::stat(
                    "File",
                    &format!("{}:{}", output::file_path(&sym.file), sym.line_start),
                );
                return;
            }
        }
    }
    // fallback
    output::stat_highlight("Symbol", symbol);
}

fn print_edge(index: usize, edge: &QueryEdge, graph: &GraphynGraph) {
    let num = output::dim(&format!("{index:>3}."));
    let location = format!("{}:{}", output::file_path(&edge.file), edge.line,);

    println!(
        "  {num} {location} {}",
        output::dim(&format!("[{}]", query::kind_name(&edge.kind)))
    );

    // alias info
    if let Some(alias) = &edge.alias {
        println!(
            "       {} imports as {}",
            output::dim("→"),
            output::alias_tag(alias),
        );
    } else {
        // show the from-symbol name if we can resolve it
        if let Some(sym) = graph.symbols.get(&edge.from) {
            println!(
                "       {} imports as {}",
                output::dim("→"),
                output::symbol_name(&sym.name),
            );
        }
    }

    // properties
    if !edge.properties_accessed.is_empty() {
        let props: Vec<String> = edge
            .properties_accessed
            .iter()
            .map(|p| output::property_name(p))
            .collect();
        println!("       {} accesses {}", output::dim("→"), props.join(", "),);
    }

    // context (truncated)
    if !edge.context.is_empty() && edge.context != "import" && edge.context != "property access" {
        let ctx = if edge.context.len() > 72 {
            format!("{}…", &edge.context[..72])
        } else {
            edge.context.clone()
        };
        println!(
            "       {} {}",
            output::dim("→"),
            output::dim(&format!("\"{}\"", ctx)),
        );
    }

    // hop distance
    if edge.hop > 1 {
        println!(
            "       {} {}",
            output::dim("→"),
            output::dim(&format!("(hop {})", edge.hop)),
        );
    }

    println!();
}

fn print_dep_edge(index: usize, edge: &QueryEdge, graph: &GraphynGraph) {
    let num = output::dim(&format!("{index:>3}."));

    // for deps, the `to` is what we depend on
    let dep_name = graph
        .symbols
        .get(&edge.to)
        .map(|s| s.name.clone())
        .unwrap_or_else(|| edge.to.clone());

    let dep_kind = graph
        .symbols
        .get(&edge.to)
        .map(|s| format_kind(&s.kind).to_string())
        .unwrap_or_default();

    let location = format!("{}:{}", output::file_path(&edge.file), edge.line,);

    println!(
        "  {num} {} {} {}",
        output::symbol_name(&dep_name),
        output::dim(&format!("[{dep_kind}]")),
        output::dim(&format!("— {location}")),
    );

    if let Some(alias) = &edge.alias {
        println!(
            "       {} via alias {}",
            output::dim("→"),
            output::alias_tag(alias),
        );
    }

    if edge.hop > 1 {
        println!(
            "       {} {}",
            output::dim("→"),
            output::dim(&format!("(hop {})", edge.hop)),
        );
    }

    println!();
}

fn collect_property_summary(edges: &[QueryEdge]) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for edge in edges {
        for prop in &edge.properties_accessed {
            *counts.entry(prop.clone()).or_insert(0) += 1;
        }
    }
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    sorted
}

fn is_aliased_only_property(edges: &[QueryEdge], property: &str) -> bool {
    edges
        .iter()
        .filter(|e| e.properties_accessed.contains(&property.to_string()))
        .all(|e| e.alias.is_some())
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
        SymbolKind::ExternalPackage => "external",
    }
}

fn format_query_error(err: graphyn_core::GraphynError, symbol: &str) -> String {
    match err {
        graphyn_core::GraphynError::SymbolNotFound(_) => {
            format!(
                "Symbol '{}' not found in the graph.\n  \
                 Hint: run `graphyn analyze` to index the repo, or check the symbol name.\n  \
                 If the symbol is a method, try: `graphyn query blast-radius ClassName`.",
                symbol
            )
        }
        graphyn_core::GraphynError::AmbiguousSymbol { candidates, .. } => {
            let files = candidates
                .iter()
                .map(|f| format!("    • {}", output::file_path(f)))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Symbol '{}' is ambiguous. Found in:\n{}\n  \
                 Hint: use --file to narrow the search.",
                symbol, files
            )
        }
        other => format!("{other}"),
    }
}
