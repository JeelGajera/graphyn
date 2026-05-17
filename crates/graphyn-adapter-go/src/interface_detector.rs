use std::collections::{HashMap, HashSet};

use graphyn_core::ir::{Relationship, RelationshipKind, RepoIR, SymbolKind};

pub fn detect_implementations(repo_ir: &mut RepoIR) {
    let mut interface_methods: HashMap<String, HashSet<String>> = HashMap::new();
    let mut struct_methods: HashMap<String, (HashSet<String>, String)> = HashMap::new();

    for file in &repo_ir.files {
        for sym in &file.symbols {
            if sym.kind == SymbolKind::Interface {
                let req = extract_method_names(sym.signature.as_deref().unwrap_or(""));
                interface_methods.insert(sym.id.clone(), req);
            }
        }
    }

    for file in &repo_ir.files {
        for sym in &file.symbols {
            if sym.kind != SymbolKind::Method {
                continue;
            }
            if let Some(receiver) = receiver_from_signature(sym.signature.as_deref().unwrap_or("")) {
                if let Some(struct_sym) = file
                    .symbols
                    .iter()
                    .find(|s| s.kind == SymbolKind::Class && s.name == receiver)
                {
                    struct_methods
                        .entry(struct_sym.id.clone())
                        .or_insert_with(|| (HashSet::new(), file.file.clone()))
                        .0
                        .insert(sym.name.clone());
                }
            }
        }
    }

    for (struct_id, (methods, struct_file)) in &struct_methods {
        for (interface_id, required) in &interface_methods {
            if required.is_empty() {
                continue;
            }
            if required.is_subset(methods) {
                if let Some(f) = repo_ir.files.iter_mut().find(|f| f.file == *struct_file) {
                    f.relationships.push(Relationship {
                        from: struct_id.clone(),
                        to: interface_id.clone(),
                        kind: RelationshipKind::Implements,
                        alias: None,
                        properties_accessed: Vec::new(),
                        context: format!("implements {} (method set match)", interface_id),
                        file: struct_file.clone(),
                        line: 0,
                    });
                }
            }
        }
    }
}

fn extract_method_names(signature: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    for token in signature.split(|c: char| c.is_whitespace() || c == '(' || c == '{' || c == ';') {
        if token.is_empty() {
            continue;
        }
        if (token
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase() || c.is_ascii_lowercase())
            .unwrap_or(false))
            && token != "type"
            && token != "interface"
            && token != "struct"
            && token != "func"
            && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            out.insert(token.to_string());
        }
    }
    out
}

fn receiver_from_signature(signature: &str) -> Option<String> {
    if !signature.starts_with("func (") {
        return None;
    }
    let inside = signature.trim_start_matches("func (").split(')').next()?;
    let ty = inside.split_whitespace().last()?.trim_start_matches('*');
    Some(ty.to_string())
}
