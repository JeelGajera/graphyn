use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use graphyn_core::ir::{
    Diagnostic, DiagnosticCategory, DiagnosticLevel, ReExportEntry, Relationship, RelationshipKind,
    RepoIR, Symbol, SymbolId,
};

use crate::extractor::{
    is_builtin_type, parse_unresolved_import_symbol_id, parse_unresolved_local_type_symbol_id,
};
use crate::tsconfig::TsConfigPaths;

// ── module specifier classification ──────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleKind {
    /// Starts with './' or '../' — must resolve to a local file
    Relative,
    /// npm package: 'react', '@org/package', 'jspdf'
    ExternalPackage,
    /// Node.js built-in: 'path', 'fs', 'os', etc.
    NodeBuiltin,
    /// Starts with '/' — absolute path (rare)
    Absolute,
}

const NODE_BUILTINS: &[&str] = &[
    "assert",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
];

pub fn classify_module_specifier(specifier: &str) -> ModuleKind {
    if specifier.starts_with("./") || specifier.starts_with("../") {
        return ModuleKind::Relative;
    }
    if specifier.starts_with('/') {
        return ModuleKind::Absolute;
    }
    // Handle 'node:fs' style imports
    let bare = specifier.strip_prefix("node:").unwrap_or(specifier);
    // Extract the package name (before any '/' subpath)
    let package_root = bare.split('/').next().unwrap_or(bare);
    if NODE_BUILTINS.contains(&package_root) {
        return ModuleKind::NodeBuiltin;
    }
    ModuleKind::ExternalPackage
}

/// Extract the package name from a module specifier.
/// `"react"` → `"react"`, `"react/jsx-runtime"` → `"react"`,
/// `"@org/pkg/sub"` → `"@org/pkg"`, `"node:fs"` → `"fs"`
fn extract_package_name(specifier: &str) -> String {
    let bare = specifier.strip_prefix("node:").unwrap_or(specifier);
    if bare.starts_with('@') {
        // Scoped: @org/pkg or @org/pkg/subpath
        let mut parts = bare.splitn(3, '/');
        let scope = parts.next().unwrap_or(bare);
        match parts.next() {
            Some(name) => format!("{scope}/{name}"),
            None => scope.to_string(),
        }
    } else {
        // Unscoped: pkg or pkg/subpath
        bare.split('/').next().unwrap_or(bare).to_string()
    }
}

pub fn resolve_repo_ir(root: &Path, repo_ir: &mut RepoIR) {
    let mut file_to_symbols: HashMap<String, Vec<Symbol>> = HashMap::new();
    let mut file_to_reexports: HashMap<String, Vec<ReExportEntry>> = HashMap::new();
    for file in &repo_ir.files {
        file_to_symbols.insert(file.file.clone(), file.symbols.clone());
        if !file.re_exports.is_empty() {
            file_to_reexports.insert(file.file.clone(), file.re_exports.clone());
        }
    }

    let tsconfig_paths = TsConfigPaths::load(root);

    for file_ir in &mut repo_ir.files {
        let mut resolved = Vec::new();
        let mut local_alias_to_symbol_id: HashMap<String, String> = HashMap::new();

        for relationship in &file_ir.relationships {
            if relationship.kind == RelationshipKind::Imports
                || relationship.kind == RelationshipKind::ReExports
            {
                let expansions = resolve_import_like(
                    root,
                    &file_ir.file,
                    relationship,
                    &file_to_symbols,
                    &file_to_reexports,
                    tsconfig_paths.as_ref(),
                    &mut file_ir.diagnostics,
                );
                for rel in expansions {
                    if relationship.kind == RelationshipKind::Imports {
                        let local_name = rel
                            .alias
                            .clone()
                            .unwrap_or_else(|| rel.to.split("::").nth(1).unwrap_or("").to_string());
                        if !local_name.is_empty() {
                            local_alias_to_symbol_id.insert(local_name, rel.to.clone());
                        }
                    }
                    resolved.push(rel);
                }
            } else {
                resolved.push(relationship.clone());
            }
        }

        for rel in &mut resolved {
            if rel.kind != RelationshipKind::AccessesProperty {
                continue;
            }
            if let Some(type_name) = parse_unresolved_local_type_symbol_id(&rel.to) {
                if let Some(canonical) = local_alias_to_symbol_id.get(&type_name) {
                    rel.to = canonical.clone();
                    continue;
                }
                if let Some(found) =
                    find_symbol_by_name_in_file(&file_to_symbols, &file_ir.file, &type_name)
                {
                    rel.to = found;
                    continue;
                }
                // Built-in types are not codebase symbols — silently skip
                if is_builtin_type(&type_name) {
                    continue;
                }
                file_ir.diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Warning,
                    category: DiagnosticCategory::Resolution,
                    message: format!(
                        "unable to resolve property-access type '{type_name}' in {}",
                        file_ir.file
                    ),
                    file: Some(file_ir.file.clone()),
                    line: None,
                });
            }
        }

        file_ir.relationships = resolved;
    }
}

fn resolve_import_like(
    root: &Path,
    file: &str,
    relationship: &Relationship,
    file_to_symbols: &HashMap<String, Vec<Symbol>>,
    file_to_reexports: &HashMap<String, Vec<ReExportEntry>>,
    tsconfig_paths: Option<&TsConfigPaths>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Relationship> {
    let local_context = LocalImportContext {
        root,
        file,
        file_to_symbols,
        file_to_reexports,
    };

    let Some((module_specifier, symbol_name)) = parse_unresolved_import_symbol_id(&relationship.to)
    else {
        return vec![relationship.clone()];
    };

    // Non-relative specifiers: try tsconfig paths first, then classify as external
    match classify_module_specifier(&module_specifier) {
        ModuleKind::ExternalPackage | ModuleKind::NodeBuiltin => {
            // Try tsconfig paths before giving up to external
            if let Some(tc) = tsconfig_paths {
                if let Some(resolved_path) = tc.resolve(root, &module_specifier) {
                    let target_file = resolved_path.to_string_lossy().replace('\\', "/");
                    return resolve_local_import(
                        &local_context,
                        &target_file,
                        &symbol_name,
                        relationship,
                        diagnostics,
                    );
                }
            }
            let package_name = extract_package_name(&module_specifier);
            let external_id = format!("ext::{package_name}::package");
            let mut rel = relationship.clone();
            rel.to = external_id;
            return vec![rel];
        }
        ModuleKind::Relative | ModuleKind::Absolute => {}
    }

    let Some(target_file) = resolve_target_file(root, file, &module_specifier) else {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            category: DiagnosticCategory::Resolution,
            message: format!(
                "unresolved import target in {file}: {}",
                relationship.context
            ),
            file: Some(file.to_string()),
            line: Some(relationship.line),
        });
        return vec![relationship.clone()];
    };

    resolve_local_import(
        &local_context,
        &target_file,
        &symbol_name,
        relationship,
        diagnostics,
    )
}

fn resolve_local_import(
    context: &LocalImportContext<'_>,
    target_file: &str,
    symbol_name: &str,
    relationship: &Relationship,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Relationship> {
    let Some(target_symbols) = context.file_to_symbols.get(target_file) else {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            category: DiagnosticCategory::Resolution,
            message: format!("target file missing symbols for import resolution: {target_file}"),
            file: Some(context.file.to_string()),
            line: None,
        });
        return vec![relationship.clone()];
    };

    if symbol_name == "*" && relationship.kind == RelationshipKind::ReExports {
        let mut out = Vec::new();
        for symbol in target_symbols {
            if symbol.name == "module" {
                continue;
            }
            let mut rel = relationship.clone();
            rel.to = symbol.id.clone();
            out.push(rel);
        }
        if out.is_empty() {
            out.push(relationship.clone());
        }
        return out;
    }

    let resolved_id = if symbol_name == "default" {
        pick_default_export_candidate(target_symbols).map(|s| s.id.clone())
    } else {
        target_symbols
            .iter()
            .find(|s| s.name == symbol_name)
            .map(|s| s.id.clone())
    };

    // If not found directly, follow re-export chain (barrel files)
    let resolved_id = resolved_id.or_else(|| {
        let mut visited = HashSet::new();
        visited.insert(target_file.to_string());
        follow_reexport_chain(
            context.root,
            target_file,
            symbol_name,
            context.file_to_symbols,
            context.file_to_reexports,
            &mut visited,
            diagnostics,
        )
    });

    let Some(resolved_id) = resolved_id else {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            category: DiagnosticCategory::Resolution,
            message: format!("unable to resolve symbol '{symbol_name}' from {target_file}"),
            file: Some(context.file.to_string()),
            line: Some(relationship.line),
        });
        return vec![relationship.clone()];
    };

    let mut rel = relationship.clone();
    rel.to = resolved_id;
    vec![rel]
}

struct LocalImportContext<'a> {
    root: &'a Path,
    file: &'a str,
    file_to_symbols: &'a HashMap<String, Vec<Symbol>>,
    file_to_reexports: &'a HashMap<String, Vec<ReExportEntry>>,
}

const MAX_REEXPORT_DEPTH: usize = 10;

fn follow_reexport_chain(
    root: &Path,
    current_file: &str,
    symbol_name: &str,
    file_to_symbols: &HashMap<String, Vec<Symbol>>,
    file_to_reexports: &HashMap<String, Vec<ReExportEntry>>,
    visited: &mut HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SymbolId> {
    if visited.len() > MAX_REEXPORT_DEPTH {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            category: DiagnosticCategory::Resolution,
            message: format!(
                "re-export chain depth exceeded for '{symbol_name}' from {current_file}"
            ),
            file: Some(current_file.to_string()),
            line: None,
        });
        return None;
    }

    let reexports = file_to_reexports.get(current_file)?;

    // Helper to follow a specific entry
    let mut try_follow = |entry: &ReExportEntry| -> Option<SymbolId> {
        let chain_target = resolve_target_file(root, current_file, &entry.source_module)?;

        // Push visited
        if !visited.insert(chain_target.clone()) {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                category: DiagnosticCategory::Resolution,
                message: format!(
                    "cyclic re-export detected for '{symbol_name}': {} → {chain_target}",
                    current_file
                ),
                file: Some(current_file.to_string()),
                line: None,
            });
            return None;
        }

        // Check if symbol exists in the direct target
        if let Some(chain_symbols) = file_to_symbols.get(&chain_target) {
            let found = if symbol_name == "default" {
                pick_default_export_candidate(chain_symbols).map(|s| s.id.clone())
            } else {
                chain_symbols
                    .iter()
                    .find(|s| s.name == symbol_name)
                    .map(|s| s.id.clone())
            };
            if found.is_some() {
                return found;
            }
        }

        // Recurse
        let result = follow_reexport_chain(
            root,
            &chain_target,
            symbol_name,
            file_to_symbols,
            file_to_reexports,
            visited,
            diagnostics,
        );

        // Pop visited to allow other branches to explore this target if needed
        // (Wait, standard DFS handles visited per path, here we use a shared visited set across all explorations
        // in this step to prevent cycles from exploding branching factor too. We can leave it inserted or pop it.
        // Popping is safer for false cycles across sibling paths.)
        visited.remove(&chain_target);

        result
    };

    // 1. Try exact matches first
    if let Some(exact_entry) = reexports.iter().find(|e| e.exported_name == symbol_name) {
        if let Some(id) = try_follow(exact_entry) {
            return Some(id);
        }
    }

    // 2. Try wildcard matches if exact match failed
    for star_entry in reexports.iter().filter(|e| e.exported_name == "*") {
        if let Some(id) = try_follow(star_entry) {
            return Some(id);
        }
    }

    None
}

fn pick_default_export_candidate(symbols: &[Symbol]) -> Option<&Symbol> {
    symbols
        .iter()
        .find(|s| s.name != "module" && !matches!(s.kind, graphyn_core::ir::SymbolKind::Property))
}

fn find_symbol_by_name_in_file(
    file_to_symbols: &HashMap<String, Vec<Symbol>>,
    file: &str,
    name: &str,
) -> Option<String> {
    let symbols = file_to_symbols.get(file)?;
    let mut candidates: Vec<&Symbol> = symbols
        .iter()
        .filter(|symbol| symbol.name == name)
        .collect();
    candidates.sort_by(|a, b| a.file.cmp(&b.file).then(a.id.cmp(&b.id)));
    candidates.first().map(|s| s.id.clone())
}

fn resolve_target_file(root: &Path, from_file: &str, module_specifier: &str) -> Option<String> {
    if !module_specifier.starts_with('.') {
        return None;
    }

    let from_file_path = root.join(from_file);
    let base_dir = from_file_path.parent()?;
    let candidate = base_dir.join(module_specifier);

    let root_canon = std::fs::canonicalize(root).ok();
    let candidates = [
        candidate.clone(),
        with_extension(&candidate, "ts"),
        with_extension(&candidate, "tsx"),
        with_extension(&candidate, "mts"),
        with_extension(&candidate, "cts"),
        with_extension(&candidate, "js"),
        with_extension(&candidate, "jsx"),
        with_extension(&candidate, "mjs"),
        with_extension(&candidate, "cjs"),
        with_extension(&candidate, "vue"),
        with_extension(&candidate, "svelte"),
        with_extension(&candidate, "astro"),
        candidate.join("index.ts"),
        candidate.join("index.tsx"),
        candidate.join("index.mts"),
        candidate.join("index.cts"),
        candidate.join("index.js"),
        candidate.join("index.jsx"),
        candidate.join("index.mjs"),
        candidate.join("index.cjs"),
        candidate.join("index.vue"),
        candidate.join("index.svelte"),
        candidate.join("index.astro"),
    ];

    for path in candidates {
        if path.is_file() {
            if let Some(rel) = relative_path_within_root(root, root_canon.as_deref(), &path) {
                return Some(rel);
            }
        }
    }

    None
}

fn with_extension(path: &Path, ext: &str) -> PathBuf {
    let mut out = path.to_path_buf();
    out.set_extension(ext);
    out
}

fn relative_path_within_root(
    root: &Path,
    root_canon: Option<&Path>,
    path: &Path,
) -> Option<String> {
    if let Some(canon_root) = root_canon {
        let canon_path = std::fs::canonicalize(path).ok()?;
        let rel = canon_path.strip_prefix(canon_root).ok()?;
        return Some(rel.to_string_lossy().replace('\\', "/"));
    }

    path.strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}
