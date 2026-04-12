use std::collections::HashMap;
use std::path::{Path, PathBuf};

use graphyn_core::ir::{Relationship, RelationshipKind, RepoIR, Symbol};

use crate::extractor::{parse_unresolved_import_symbol_id, parse_unresolved_local_type_symbol_id};

pub fn resolve_repo_ir(root: &Path, repo_ir: &mut RepoIR) {
    let mut file_to_symbols: HashMap<String, Vec<Symbol>> = HashMap::new();
    for file in &repo_ir.files {
        file_to_symbols.insert(file.file.clone(), file.symbols.clone());
    }

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
                    &mut file_ir.parse_errors,
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
                if let Some(found) = find_symbol_by_name_anywhere(&file_to_symbols, &type_name) {
                    rel.to = found;
                }
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
    parse_errors: &mut Vec<String>,
) -> Vec<Relationship> {
    let Some((module_specifier, symbol_name)) = parse_unresolved_import_symbol_id(&relationship.to)
    else {
        return vec![relationship.clone()];
    };

    let Some(target_file) = resolve_target_file(root, file, &module_specifier) else {
        parse_errors.push(format!(
            "unresolved import target in {file}: {}",
            relationship.context
        ));
        return vec![relationship.clone()];
    };

    let Some(target_symbols) = file_to_symbols.get(&target_file) else {
        parse_errors.push(format!(
            "target file missing symbols for import resolution: {target_file}"
        ));
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

    let Some(resolved_id) = resolved_id else {
        parse_errors.push(format!(
            "unable to resolve symbol '{symbol_name}' from {target_file}"
        ));
        return vec![relationship.clone()];
    };

    let mut rel = relationship.clone();
    rel.to = resolved_id;
    vec![rel]
}

fn pick_default_export_candidate(symbols: &[Symbol]) -> Option<&Symbol> {
    symbols
        .iter()
        .find(|s| s.name != "module" && !matches!(s.kind, graphyn_core::ir::SymbolKind::Property))
}

fn find_symbol_by_name_anywhere(
    file_to_symbols: &HashMap<String, Vec<Symbol>>,
    name: &str,
) -> Option<String> {
    let mut candidates: Vec<&Symbol> = file_to_symbols
        .values()
        .flat_map(|symbols| symbols.iter())
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

    let candidates = [
        candidate.clone(),
        with_extension(&candidate, "ts"),
        with_extension(&candidate, "tsx"),
        with_extension(&candidate, "js"),
        with_extension(&candidate, "jsx"),
        candidate.join("index.ts"),
        candidate.join("index.tsx"),
        candidate.join("index.js"),
        candidate.join("index.jsx"),
    ];

    for path in candidates {
        if path.is_file() {
            return path.strip_prefix(root).ok().map(normalize_relative_path);
        }
    }

    None
}

fn with_extension(path: &Path, ext: &str) -> PathBuf {
    let mut out = path.to_path_buf();
    out.set_extension(ext);
    out
}

fn normalize_relative_path(path: &Path) -> String {
    let mut stack: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                let _ = stack.pop();
            }
            std::path::Component::Normal(part) => {
                stack.push(part.to_string_lossy().to_string());
            }
            _ => {}
        }
    }
    stack.join("/")
}
