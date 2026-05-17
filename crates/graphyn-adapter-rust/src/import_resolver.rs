use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use graphyn_core::ir::{RelationshipKind, RepoIR};

use crate::module_tree::ModuleTree;

pub fn resolve_repo_ir(_root: &Path, repo_ir: &mut RepoIR) {
    let module_tree = ModuleTree::build(&repo_ir.files);

    for f in &mut repo_ir.files {
        let mut local_name_to_symbol_id: HashMap<String, String> = HashMap::new();
        let prop_edges = f
            .relationships
            .iter()
            .filter(|r| r.kind == RelationshipKind::AccessesProperty)
            .cloned()
            .collect::<Vec<_>>();

        for r in &mut f.relationships {
            if r.kind != RelationshipKind::Imports && r.kind != RelationshipKind::Implements {
                continue;
            }
            if !r.to.starts_with("unresolved_import::") {
                continue;
            }

            let raw = r.to.trim_start_matches("unresolved_import::").to_string();
            let is_local = raw.starts_with("crate::")
                || raw.starts_with("super::")
                || raw.starts_with("self::")
                || module_tree.could_be_local(&raw);

            if is_local {
                if let Some(id) = module_tree.resolve_use_path(&f.file, &raw) {
                    r.to = id.clone();
                    let local = r
                        .alias
                        .clone()
                        .unwrap_or_else(|| raw.rsplit("::").next().unwrap_or("").to_string());
                    local_name_to_symbol_id.insert(local, id);
                } else {
                    r.to = format!("unresolved_local::{raw}");
                }
            } else {
                let pkg = raw.split("::").next().unwrap_or("ext");
                r.to = format!("ext::{}::package", pkg);
            }
        }

        for r in &mut f.relationships {
            if r.kind != RelationshipKind::Imports {
                continue;
            }
            if r.to.starts_with("unresolved_") || r.to.starts_with("ext::") {
                continue;
            }

            let local_name = r
                .alias
                .clone()
                .unwrap_or_else(|| r.to.split("::").nth(1).unwrap_or("").to_string());

            let mut props = BTreeSet::new();
            for p in &prop_edges {
                let var = p.to.trim_start_matches("unresolved_local_type::");
                if var == "data" || var == local_name || local_name_to_symbol_id.contains_key(var) {
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
