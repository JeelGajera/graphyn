use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use graphyn_core::ir::{Relationship, RelationshipKind, RepoIR, SymbolKind};

fn kind_suffix(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::TypeAlias => "type_alias",
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Property => "property",
        SymbolKind::Variable => "variable",
        SymbolKind::Module => "module",
        SymbolKind::Enum => "enum",
        SymbolKind::EnumVariant => "enum_variant",
        SymbolKind::ExternalPackage => "package",
    }
}

fn make_symbol_id(file: &str, name: &str, kind: &SymbolKind) -> String {
    format!("{}::{}::{}", file, name, kind_suffix(kind))
}

fn file_to_module(file: &str) -> String {
    file.trim_end_matches(".py")
        .trim_end_matches("/__init__")
        .replace('/', ".")
}

fn resolve_relative_module(from_file: &str, module_spec: &str) -> String {
    if !module_spec.starts_with('.') {
        return module_spec.to_string();
    }

    let dots = module_spec.chars().take_while(|c| *c == '.').count();
    let suffix = &module_spec[dots..];

    let mut parts: Vec<&str> = from_file.split('/').collect();
    if !parts.is_empty() {
        parts.pop();
    }

    for _ in 0..dots.saturating_sub(1) {
        if !parts.is_empty() {
            parts.pop();
        }
    }

    let base = parts.join(".");
    if suffix.is_empty() {
        base
    } else if base.is_empty() {
        suffix.to_string()
    } else {
        format!("{base}.{suffix}")
    }
}

pub fn resolve_repo_ir(_root: &Path, repo_ir: &mut RepoIR) {
    let mut class_by_module_and_name: HashMap<(String, String), String> = HashMap::new();
    let mut init_reexports: HashMap<String, Vec<String>> = HashMap::new();

    for file in &repo_ir.files {
        let module = file_to_module(&file.file);
        for sym in &file.symbols {
            if sym.kind == SymbolKind::Class || sym.kind == SymbolKind::Interface {
                class_by_module_and_name.insert((module.clone(), sym.name.clone()), sym.id.clone());
            }
        }
        if file.file.ends_with("/__init__.py") {
            let exports = file
                .relationships
                .iter()
                .filter(|r| r.kind == RelationshipKind::Imports)
                .filter_map(|r| r.alias.clone().or_else(|| r.to.rsplit("::").next().map(|s| s.to_string())))
                .collect::<Vec<_>>();
            init_reexports.insert(module, exports);
        }
    }

    for file in &mut repo_ir.files {
        let mut local_name_to_symbol_id: HashMap<String, String> = HashMap::new();
        let property_edges: Vec<Relationship> = file
            .relationships
            .iter()
            .filter(|r| r.kind == RelationshipKind::AccessesProperty)
            .cloned()
            .collect();

        for rel in &mut file.relationships {
            if rel.kind != RelationshipKind::Imports || !rel.to.starts_with("unresolved_import::") {
                continue;
            }

            let raw = rel.to.trim_start_matches("unresolved_import::").to_string();
            let mut parts = raw.splitn(2, "::");
            let module_raw = parts.next().unwrap_or("");
            let symbol = parts.next().unwrap_or("*").to_string();
            let module = resolve_relative_module(&file.file, module_raw);

            if symbol == "*" {
                rel.to = format!("ext::{}::package", module.split('.').next().unwrap_or("unknown"));
                continue;
            }

            let mut resolved: Option<String> = class_by_module_and_name
                .get(&(module.clone(), symbol.clone()))
                .cloned();

            if resolved.is_none() {
                let init_file_mod = module.clone();
                if init_reexports
                    .get(&init_file_mod)
                    .map(|xs| xs.iter().any(|x| x == &symbol))
                    .unwrap_or(false)
                {
                    resolved = class_by_module_and_name
                        .iter()
                        .find(|((m, n), _)| m.starts_with(&(module.clone() + ".")) && *n == symbol)
                        .map(|(_, id)| id.clone());
                }
            }

            if let Some(id) = resolved {
                rel.to = id.clone();
                let local_name = rel.alias.clone().unwrap_or_else(|| symbol.clone());
                local_name_to_symbol_id.insert(local_name, id);
            } else {
                rel.to = format!("ext::{}::package", module.split('.').next().unwrap_or("unknown"));
            }
        }

        for rel in &mut file.relationships {
            if rel.kind != RelationshipKind::Imports || rel.to.starts_with("ext::") {
                continue;
            }
            let local_name = rel
                .alias
                .clone()
                .unwrap_or_else(|| rel.to.split("::").nth(1).unwrap_or("").to_string());

            let mut props = BTreeSet::new();
            for edge in &property_edges {
                if !edge.to.starts_with("unresolved_local_type::") {
                    continue;
                }
                let ty = edge.to.trim_start_matches("unresolved_local_type::");
                let applies = ty == local_name
                    || local_name_to_symbol_id
                        .get(ty)
                        .map(|id| id == &rel.to)
                        .unwrap_or(false);
                if applies {
                    for p in &edge.properties_accessed {
                        props.insert(p.clone());
                    }
                }
            }
            rel.properties_accessed = props.into_iter().collect();
        }

        file.relationships
            .retain(|r| r.kind != RelationshipKind::AccessesProperty);
    }

    for file in &mut repo_ir.files {
        if !file
            .symbols
            .iter()
            .any(|s| s.id == make_symbol_id(&file.file, "module", &SymbolKind::Module))
        {
            file.symbols.push(graphyn_core::ir::Symbol {
                id: make_symbol_id(&file.file, "module", &SymbolKind::Module),
                name: "module".to_string(),
                kind: SymbolKind::Module,
                language: graphyn_core::ir::Language::Python,
                file: file.file.clone(),
                line_start: 1,
                line_end: 1,
                signature: None,
            });
        }
    }
}
