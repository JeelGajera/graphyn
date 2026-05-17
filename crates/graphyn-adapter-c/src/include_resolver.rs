use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use graphyn_core::ir::{RelationshipKind, RepoIR, SymbolKind};

pub fn resolve_repo_ir(_root: &Path, repo_ir: &mut RepoIR) {
    let mut by_name: HashMap<String, String> = HashMap::new();
    let mut header_to_symbols: HashMap<String, Vec<String>> = HashMap::new();

    for f in &repo_ir.files {
        for s in &f.symbols {
            by_name.entry(s.name.clone()).or_insert_with(|| s.id.clone());
        }
        let filename = Path::new(&f.file)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ids: Vec<String> = f
            .symbols
            .iter()
            .filter(|s| s.kind != SymbolKind::Module)
            .map(|s| s.id.clone())
            .collect();
        header_to_symbols.insert(filename, ids.clone());
        header_to_symbols.insert(f.file.clone(), ids);
    }

    for f in &mut repo_ir.files {
        let prop_edges = f
            .relationships
            .iter()
            .filter(|r| r.kind == RelationshipKind::AccessesProperty)
            .cloned()
            .collect::<Vec<_>>();

        let mut alias_to_symbol: HashMap<String, String> = HashMap::new();

        for r in &mut f.relationships {
            if r.kind != RelationshipKind::Imports && r.kind != RelationshipKind::Extends {
                continue;
            }

            if r.to.starts_with("unresolved_include::") {
                let raw = r.to.trim_start_matches("unresolved_include::");
                let path = raw
                    .trim_start_matches("#include")
                    .trim()
                    .trim_matches('"')
                    .trim_matches('<')
                    .trim_matches('>');
                let filename = Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());

                if header_to_symbols.contains_key(&filename) {
                    r.to = format!("local_header::{}", filename);
                } else {
                    r.to = format!("ext::{}::package", path.replace(['/', '.'], "_"));
                }
            } else if r.to.starts_with("unresolved_alias::") {
                let base = r.to.trim_start_matches("unresolved_alias::").to_string();
                if let Some(id) = by_name.get(&base) {
                    r.to = id.clone();
                    if let Some(alias) = &r.alias {
                        alias_to_symbol.insert(alias.clone(), id.clone());
                    }
                }
            }
        }

        for r in &mut f.relationships {
            if r.kind != RelationshipKind::Imports {
                continue;
            }
            if r.to.starts_with("unresolved_") || r.to.starts_with("ext::") {
                continue;
            }

            let mut props = BTreeSet::new();
            if let Some(alias) = &r.alias {
                for p in &prop_edges {
                    let obj = p.to.trim_start_matches("unresolved_local_type::");
                    if obj == "data" || obj == alias || alias_to_symbol.contains_key(obj) {
                        for fld in &p.properties_accessed {
                            props.insert(fld.clone());
                        }
                    }
                }
            }
            r.properties_accessed = props.into_iter().collect();
        }

        f.relationships
            .retain(|r| r.kind != RelationshipKind::AccessesProperty);
    }
}
