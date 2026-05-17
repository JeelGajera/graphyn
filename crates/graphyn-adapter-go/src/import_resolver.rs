use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use graphyn_core::ir::{RelationshipKind, RepoIR, SymbolKind};

use crate::module_resolver::GoModule;

pub fn resolve_repo_ir(root: &Path, repo_ir: &mut RepoIR) {
    let module = GoModule::load(root);

    let mut pkg_to_symbols: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(ref m) = module {
        for f in &repo_ir.files {
            let dir = Path::new(&f.file)
                .parent()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if !dir.is_empty() {
                let pkg_path = format!("{}/{}", m.module_path, dir);
                for s in &f.symbols {
                    if s.kind != SymbolKind::Module {
                        pkg_to_symbols
                            .entry(pkg_path.clone())
                            .or_default()
                            .push(s.id.clone());
                    }
                }
            }
        }
    }

    for f in &mut repo_ir.files {
        let prop_edges = f
            .relationships
            .iter()
            .filter(|r| r.kind == RelationshipKind::AccessesProperty)
            .cloned()
            .collect::<Vec<_>>();

        let mut alias_to_pkg: HashMap<String, String> = HashMap::new();

        for r in &mut f.relationships {
            if r.kind != RelationshipKind::Imports || !r.to.starts_with("unresolved_import::") {
                continue;
            }

            let pkg_path = r.to.trim_start_matches("unresolved_import::").to_string();
            let leaf = pkg_path
                .rsplit('/')
                .next()
                .unwrap_or(pkg_path.as_str())
                .to_string();
            let local_alias = r.alias.clone().unwrap_or_else(|| leaf.clone());

            let is_local = module
                .as_ref()
                .map(|m| pkg_path.starts_with(&m.module_path))
                .unwrap_or(false);

            if is_local {
                if let Some(symbol_ids) = pkg_to_symbols.get(&pkg_path) {
                    r.to = symbol_ids
                        .first()
                        .cloned()
                        .unwrap_or_else(|| format!("ext::{}::package", leaf));
                } else {
                    r.to = format!("ext::{}::package", leaf);
                }
                alias_to_pkg.insert(local_alias.clone(), pkg_path.clone());
            } else {
                r.to = format!("ext::{}::package", leaf);
            }
        }

        for r in &mut f.relationships {
            if r.kind != RelationshipKind::Imports || r.to.starts_with("ext::") {
                continue;
            }
            let local_alias = r
                .alias
                .clone()
                .unwrap_or_else(|| r.to.split("::").nth(1).unwrap_or("").to_string());
            let mut props = BTreeSet::new();
            for p in &prop_edges {
                let obj = p.to.trim_start_matches("unresolved_local_type::");
                if obj == "data" || obj == local_alias || alias_to_pkg.contains_key(obj) {
                    for fld in &p.properties_accessed {
                        props.insert(fld.clone());
                    }
                }
            }
            r.properties_accessed = props.into_iter().collect();
        }

        f.relationships
            .retain(|r| r.kind != RelationshipKind::AccessesProperty);
    }
}
