use std::collections::{BTreeMap, BTreeSet};

use graphyn_core::ir::{
    FileIR, Language, ReExportEntry, Relationship, RelationshipKind, Symbol, SymbolKind,
};
use regex::Regex;
use tree_sitter::Node;

use crate::parser::ParsedFile;

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

fn module_symbol(file: &str, language: Language) -> Symbol {
    Symbol {
        id: make_symbol_id(file, "module", &SymbolKind::Module),
        name: "module".to_string(),
        kind: SymbolKind::Module,
        language,
        file: file.to_string(),
        line_start: 1,
        line_end: 1,
        signature: None,
    }
}

pub fn extract_file_ir(parsed: &ParsedFile) -> FileIR {
    let source = parsed.source.as_bytes();
    let root = parsed.tree.root_node();

    let mut symbols = vec![module_symbol(&parsed.file, parsed.language.clone())];
    let mut relationships = Vec::new();
    let mut re_exports = Vec::new();

    walk_tree(root, &mut |node| match node.kind() {
        "class_definition" => {
            if let Some(sym) = extract_class(node, source, &parsed.file, parsed.language.clone()) {
                symbols.push(sym);
            }
        }
        "function_definition" => {
            if let Some(sym) = extract_function(node, source, &parsed.file, parsed.language.clone()) {
                symbols.push(sym);
            }
        }
        "import_statement" => relationships.extend(extract_plain_import(node, source, &parsed.file)),
        "import_from_statement" => {
            relationships.extend(extract_from_import(node, source, &parsed.file))
        }
        "expression_statement" => {
            if let Some(entries) = extract_all_list(node, source) {
                re_exports.extend(entries);
            }
        }
        _ => {}
    });

    relationships.extend(extract_property_accesses(
        root,
        source,
        &parsed.file,
        parsed.language.clone(),
    ));

    FileIR {
        file: parsed.file.clone(),
        language: parsed.language.clone(),
        symbols,
        relationships,
        diagnostics: parsed.diagnostics.clone(),
        re_exports,
    }
}

fn extract_class(node: Node<'_>, source: &[u8], file: &str, lang: Language) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;

    let kind = if let Some(args) = node.child_by_field_name("superclasses") {
        let superclass_text = node_text(args, source).unwrap_or("");
        if superclass_text.contains("Protocol")
            || superclass_text.contains("ABC")
            || superclass_text.contains("ABCMeta")
        {
            SymbolKind::Interface
        } else {
            SymbolKind::Class
        }
    } else {
        SymbolKind::Class
    };

    Some(Symbol {
        id: make_symbol_id(file, name, &kind),
        name: name.to_string(),
        kind,
        language: lang,
        file: file.to_string(),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        signature: Some(
            node_text(node, source)
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string(),
        ),
    })
}

fn extract_function(node: Node<'_>, source: &[u8], file: &str, lang: Language) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    let is_method = node
        .parent()
        .map(|p| p.kind() == "block" && p.parent().map(|pp| pp.kind()) == Some("class_definition"))
        .unwrap_or(false);
    let kind = if is_method { SymbolKind::Method } else { SymbolKind::Function };

    Some(Symbol {
        id: make_symbol_id(file, name, &kind),
        name: name.to_string(),
        kind,
        language: lang,
        file: file.to_string(),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        signature: Some(
            node_text(node, source)
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string(),
        ),
    })
}

fn extract_plain_import(node: Node<'_>, source: &[u8], file: &str) -> Vec<Relationship> {
    let text = node_text(node, source).unwrap_or("");
    let line = node.start_position().row as u32 + 1;
    let mut out = Vec::new();

    let rest = text.trim_start_matches("import ");
    for item in rest.split(',') {
        let bit = item.trim();
        if bit.is_empty() {
            continue;
        }
        let (name, alias) = if let Some((n, a)) = bit.split_once(" as ") {
            (n.trim(), Some(a.trim().to_string()))
        } else {
            (bit, None)
        };
        out.push(Relationship {
            from: make_symbol_id(file, "module", &SymbolKind::Module),
            to: format!("unresolved_import::{}::*", name),
            kind: RelationshipKind::Imports,
            alias,
            properties_accessed: Vec::new(),
            context: text.to_string(),
            file: file.to_string(),
            line,
        });
    }
    out
}

fn extract_from_import(node: Node<'_>, source: &[u8], file: &str) -> Vec<Relationship> {
    let text = node_text(node, source).unwrap_or("");
    let compact = text
        .replace(['\n', '\r', '\t'], " ")
        .replace(['(', ')'], " ");
    let line = node.start_position().row as u32 + 1;
    let mut out = Vec::new();

    let re = Regex::new(r"^\s*from\s+([^\s]+)\s+import\s+(.+)$").expect("valid regex");
    if let Some(caps) = re.captures(compact.trim()) {
        let module = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let imports = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        for item in imports.split(',') {
            let bit = item.trim();
            if bit.is_empty() {
                continue;
            }
            if bit == "*" {
                out.push(Relationship {
                    from: make_symbol_id(file, "module", &SymbolKind::Module),
                    to: format!("unresolved_import::{}::*", module),
                    kind: RelationshipKind::Imports,
                    alias: None,
                    properties_accessed: Vec::new(),
                    context: text.to_string(),
                    file: file.to_string(),
                    line,
                });
                continue;
            }

            let (name, alias) = if let Some((n, a)) = bit.split_once(" as ") {
                (n.trim(), Some(a.trim().to_string()))
            } else {
                (bit, None)
            };
            out.push(Relationship {
                from: make_symbol_id(file, "module", &SymbolKind::Module),
                to: format!("unresolved_import::{}::{}", module, name),
                kind: RelationshipKind::Imports,
                alias,
                properties_accessed: Vec::new(),
                context: text.to_string(),
                file: file.to_string(),
                line,
            });
        }
    }

    out
}

fn extract_property_accesses(
    root: Node<'_>,
    source: &[u8],
    file: &str,
    _lang: Language,
) -> Vec<Relationship> {
    let mut out = Vec::new();
    let param_re = Regex::new(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([A-Za-z_][A-Za-z0-9_]*)")
        .expect("valid regex");
    let attr_re = Regex::new(r"([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)")
        .expect("valid regex");

    walk_tree(root, &mut |node| {
        if node.kind() != "function_definition" {
            return;
        }

        let mut var_to_type: BTreeMap<String, String> = BTreeMap::new();
        if let Some(params) = node.child_by_field_name("parameters").and_then(|n| node_text(n, source)) {
            for cap in param_re.captures_iter(params) {
                let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let ty = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                if !name.is_empty() && !ty.is_empty() && name != "self" {
                    var_to_type.insert(name.to_string(), ty.to_string());
                }
            }
        }

        if var_to_type.is_empty() {
            return;
        }

        let mut type_to_props: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let fn_text = node_text(node, source).unwrap_or("");
        for cap in attr_re.captures_iter(fn_text) {
            let obj = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let attr = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            if let Some(ty) = var_to_type.get(obj) {
                if !attr.is_empty() {
                    type_to_props.entry(ty.clone()).or_default().insert(attr.to_string());
                }
            }
        }

        for (ty, props) in type_to_props {
            out.push(Relationship {
                from: make_symbol_id(file, "module", &SymbolKind::Module),
                to: format!("unresolved_local_type::{}", ty),
                kind: RelationshipKind::AccessesProperty,
                alias: Some(ty),
                properties_accessed: props.into_iter().collect(),
                context: "property access".to_string(),
                file: file.to_string(),
                line: node.start_position().row as u32 + 1,
            });
        }
    });

    out
}

fn extract_all_list(node: Node<'_>, source: &[u8]) -> Option<Vec<ReExportEntry>> {
    let text = node_text(node, source)?;
    if !text.trim_start().starts_with("__all__") {
        return None;
    }
    let inside = text.split('[').nth(1)?.split(']').next()?;
    let mut out = Vec::new();
    for part in inside.split(',') {
        let name = part.trim().trim_matches('"').trim_matches('\'');
        if !name.is_empty() {
            out.push(ReExportEntry {
                exported_name: name.to_string(),
                source_module: ".".to_string(),
            });
        }
    }
    Some(out)
}

fn walk_tree<F>(node: Node<'_>, f: &mut F)
where
    F: FnMut(Node<'_>),
{
    f(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_tree(child, f);
    }
}

fn node_text<'a>(node: Node<'_>, source: &'a [u8]) -> Option<&'a str> {
    node.utf8_text(source).ok()
}
