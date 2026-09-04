//! Turning placeholder ids into graph-addressable ones.
//!
//! Two passes over each file, in this order because the second depends on the
//! first: resolve imports to build the file's local-name table, then resolve
//! type references against that table.
//!
//! Anything still unresolved at the end is removed and reported. That matters
//! because [`graphyn_core::graph::GraphynGraph::add_relationship`] silently
//! drops edges pointing at ids it does not know — so an unresolved placeholder
//! that reaches the graph is a missing edge nobody hears about. Reporting it
//! here turns a silent gap into a diagnostic the user can act on.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use graphyn_core::ir::{
    Diagnostic, DiagnosticCategory, DiagnosticLevel, RelationshipKind, RepoIR, SymbolKind,
};
use graphyn_core::symbol_id::{
    external_package_id, parse_unresolved_import_id, parse_unresolved_local_type_id,
};

use crate::module_tree::{ModuleTree, Resolved};
use crate::scope_analyzer::is_builtin_type;

pub fn resolve_repo_ir(root: &Path, repo_ir: &mut RepoIR) {
    let tree = ModuleTree::build(root, &repo_ir.files);

    for file in &mut repo_ir.files {
        let path = file.file.clone();

        // Symbols this file defines, for resolving references to its own types.
        let own_symbols: HashMap<String, String> = file
            .symbols
            .iter()
            .filter(|s| s.kind != SymbolKind::Module)
            .map(|s| (s.name.clone(), s.id.clone()))
            .collect();

        // ── pass 1: imports ──────────────────────────────────
        //
        // Populates the table that pass 2 resolves type names against.
        let mut local_names: HashMap<String, String> = HashMap::new();
        let mut drop_import = Vec::new();

        for (index, rel) in file.relationships.iter_mut().enumerate() {
            if rel.kind != RelationshipKind::Imports {
                continue;
            }
            let Some((module, symbol)) = parse_unresolved_import_id(&rel.to) else {
                continue;
            };
            let (module, symbol) = (module.to_string(), symbol.to_string());

            // The name this import is known by for the rest of the file.
            let local = rel.alias.clone().unwrap_or_else(|| {
                if symbol == graphyn_core::symbol_id::IMPORT_ALL {
                    last_segment(&module).to_string()
                } else {
                    symbol.clone()
                }
            });

            match tree.resolve(&path, &module, &symbol) {
                Resolved::Symbol(id) | Resolved::Module(id) => {
                    rel.to = id.clone();
                    local_names.insert(local, id);
                }
                Resolved::External(package) => {
                    let id = external_package_id(&package);
                    rel.to = id.clone();
                    local_names.insert(local, id);
                }
                Resolved::UnknownMember => {
                    // The module exists but does not export this name. Keep the
                    // file-level dependency, which is still true, and say so.
                    match tree.resolve(&path, &module, graphyn_core::symbol_id::IMPORT_ALL) {
                        Resolved::Module(module_id) => {
                            rel.to = module_id.clone();
                            local_names.insert(local, module_id);
                        }
                        _ => drop_import.push(index),
                    }
                    file.diagnostics.push(Diagnostic {
                        level: DiagnosticLevel::Warning,
                        category: DiagnosticCategory::Resolution,
                        message: format!(
                            "module '{module}' does not export '{symbol}'; \
                             recorded a dependency on the module instead"
                        ),
                        file: Some(path.clone()),
                        line: Some(rel.line),
                    });
                }
                Resolved::Unknown => {
                    drop_import.push(index);
                    file.diagnostics.push(Diagnostic {
                        level: DiagnosticLevel::Warning,
                        category: DiagnosticCategory::Resolution,
                        message: format!("unable to resolve import '{module}::{symbol}'"),
                        file: Some(path.clone()),
                        line: Some(rel.line),
                    });
                }
            }
        }

        // ── pass 2: type references ──────────────────────────
        //
        // `AccessesProperty` from field access, `Implements` from `impl … for`
        // and `#[derive(..)]`. Both carry the type name in the placeholder.
        let mut drop_type = Vec::new();
        let mut resolved_props: HashMap<String, BTreeSet<String>> = HashMap::new();

        for (index, rel) in file.relationships.iter_mut().enumerate() {
            let Some(type_name) = parse_unresolved_local_type_id(&rel.to).map(str::to_string)
            else {
                continue;
            };

            let resolved = local_names
                .get(&type_name)
                .or_else(|| own_symbols.get(&type_name))
                .cloned();

            match resolved {
                Some(id) => {
                    if rel.kind == RelationshipKind::AccessesProperty {
                        resolved_props
                            .entry(id.clone())
                            .or_default()
                            .extend(rel.properties_accessed.iter().cloned());
                    }
                    rel.to = id;
                }
                None => {
                    drop_type.push(index);
                    // Standard-library types are genuine references to code
                    // outside the repository, not resolution failures.
                    if !is_builtin_type(&type_name) {
                        file.diagnostics.push(Diagnostic {
                            level: DiagnosticLevel::Warning,
                            category: DiagnosticCategory::Resolution,
                            message: format!(
                                "unable to resolve type '{type_name}' referenced in {path}"
                            ),
                            file: Some(path.clone()),
                            line: Some(rel.line),
                        });
                    }
                }
            }
        }

        // ── pass 3: attribute properties to their import ─────
        //
        // Keyed by resolved target, so a file that touches two imported types
        // gives each one only its own fields.
        for rel in &mut file.relationships {
            if rel.kind != RelationshipKind::Imports {
                continue;
            }
            if let Some(props) = resolved_props.get(&rel.to) {
                rel.properties_accessed = props.iter().cloned().collect();
            }
        }

        let mut unresolved: BTreeSet<usize> = drop_import.into_iter().collect();
        unresolved.extend(drop_type);
        if !unresolved.is_empty() {
            let mut index = 0usize;
            file.relationships.retain(|_| {
                let keep = !unresolved.contains(&index);
                index += 1;
                keep
            });
        }
    }
}

fn last_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}
