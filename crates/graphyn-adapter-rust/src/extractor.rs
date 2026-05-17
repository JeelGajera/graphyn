use graphyn_core::ir::{FileIR, Language, Relationship, RelationshipKind, Symbol, SymbolKind};
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

fn module_symbol(file: &str, lang: Language) -> Symbol {
    Symbol {
        id: make_symbol_id(file, "module", &SymbolKind::Module),
        name: "module".to_string(),
        kind: SymbolKind::Module,
        language: lang,
        file: file.to_string(),
        line_start: 1,
        line_end: 1,
        signature: None,
    }
}

pub fn extract_file_ir(parsed: &ParsedFile) -> FileIR {
    let src = parsed.source.as_bytes();
    let root = parsed.tree.root_node();

    let mut symbols = vec![module_symbol(&parsed.file, parsed.language.clone())];
    let mut relationships = Vec::new();

    walk_tree(root, &mut |node| match node.kind() {
        "struct_item" => {
            if let Some(sym) = extract_named_item(node, src, &parsed.file, parsed.language.clone(), SymbolKind::Class) {
                symbols.push(sym);
            }
        }
        "enum_item" => {
            if let Some(sym) = extract_named_item(node, src, &parsed.file, parsed.language.clone(), SymbolKind::Enum) {
                symbols.push(sym);
            }
            symbols.extend(extract_enum_variants(node, src, &parsed.file, parsed.language.clone()));
        }
        "trait_item" => {
            if let Some(sym) = extract_named_item(node, src, &parsed.file, parsed.language.clone(), SymbolKind::Interface) {
                symbols.push(sym);
            }
        }
        "function_item" => {
            let parent_kind = node.parent().map(|p| p.kind()).unwrap_or("");
            if parent_kind == "declaration_list" {
                return;
            }
            if let Some(sym) = extract_named_item(node, src, &parsed.file, parsed.language.clone(), SymbolKind::Function) {
                symbols.push(sym);
            }
        }
        "impl_item" => {
            relationships.extend(extract_impl_block(node, src, &parsed.file, parsed.language.clone(), &mut symbols));
        }
        "use_declaration" => {
            relationships.extend(extract_use_declaration(node, src, &parsed.file));
        }
        "field_expression" => {
            if let (Some(value), Some(field)) = (node.child_by_field_name("value"), node.child_by_field_name("field")) {
                let obj = node_text(value, src).unwrap_or("");
                let fld = node_text(field, src).unwrap_or("");
                if !obj.is_empty() && !fld.is_empty() {
                    relationships.push(Relationship {
                        from: make_symbol_id(&parsed.file, "module", &SymbolKind::Module),
                        to: format!("unresolved_local_type::{}", obj),
                        kind: RelationshipKind::AccessesProperty,
                        alias: Some(obj.to_string()),
                        properties_accessed: vec![fld.to_string()],
                        context: "property access".to_string(),
                        file: parsed.file.clone(),
                        line: node.start_position().row as u32 + 1,
                    });
                }
            }
        }
        _ => {}
    });

    FileIR {
        file: parsed.file.clone(),
        language: parsed.language.clone(),
        symbols,
        relationships,
        diagnostics: Vec::new(),
        re_exports: Vec::new(),
    }
}

fn extract_named_item(
    node: Node<'_>,
    source: &[u8],
    file: &str,
    lang: Language,
    kind: SymbolKind,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    Some(Symbol {
        id: make_symbol_id(file, name, &kind),
        name: name.to_string(),
        kind,
        language: lang,
        file: file.to_string(),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        signature: Some(node_text(node, source).unwrap_or("").lines().next().unwrap_or("").to_string()),
    })
}

fn extract_enum_variants(node: Node<'_>, source: &[u8], file: &str, lang: Language) -> Vec<Symbol> {
    let mut out = Vec::new();
    walk_tree(node, &mut |child| {
        if child.kind() == "enum_variant" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text(name_node, source).unwrap_or("");
                if !name.is_empty() {
                    out.push(Symbol {
                        id: make_symbol_id(file, name, &SymbolKind::EnumVariant),
                        name: name.to_string(),
                        kind: SymbolKind::EnumVariant,
                        language: lang.clone(),
                        file: file.to_string(),
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        signature: None,
                    });
                }
            }
        }
    });
    out
}

fn extract_use_declaration(node: Node<'_>, source: &[u8], file: &str) -> Vec<Relationship> {
    let mut out = Vec::new();
    let line = node.start_position().row as u32 + 1;
    let context = node_text(node, source).unwrap_or("").to_string();

    if let Some(use_tree) = node.child_by_field_name("argument") {
        collect_use_tree_items(use_tree, source, "", file, line, &context, &mut out);
    }
    out
}

fn collect_use_tree_items(
    node: Node<'_>,
    source: &[u8],
    prefix: &str,
    file: &str,
    line: u32,
    context: &str,
    out: &mut Vec<Relationship>,
) {
    match node.kind() {
        "use_as_clause" => {
            let path_node = node.child_by_field_name("path");
            let alias_node = node.child_by_field_name("alias");
            if let (Some(p), Some(a)) = (path_node, alias_node) {
                let path = node_text(p, source).unwrap_or("");
                let alias = node_text(a, source).unwrap_or("");
                let full_path = if prefix.is_empty() { path.to_string() } else { format!("{}::{}", prefix, path) };
                out.push(Relationship {
                    from: make_symbol_id(file, "module", &SymbolKind::Module),
                    to: format!("unresolved_import::{}", full_path),
                    kind: RelationshipKind::Imports,
                    alias: Some(alias.to_string()),
                    properties_accessed: Vec::new(),
                    context: context.to_string(),
                    file: file.to_string(),
                    line,
                });
            }
        }
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if !matches!(child.kind(), "{" | "}" | ",") {
                    collect_use_tree_items(child, source, prefix, file, line, context, out);
                }
            }
        }
        "scoped_use_list" => {
            let path = node.child_by_field_name("path").and_then(|p| node_text(p, source)).unwrap_or("");
            let new_prefix = if prefix.is_empty() { path.to_string() } else { format!("{}::{}", prefix, path) };
            if let Some(list) = node.child_by_field_name("list") {
                collect_use_tree_items(list, source, &new_prefix, file, line, context, out);
            }
        }
        "use_wildcard" => {
            let full = if prefix.is_empty() { "*".to_string() } else { format!("{}::*", prefix) };
            out.push(Relationship {
                from: make_symbol_id(file, "module", &SymbolKind::Module),
                to: format!("unresolved_import::{}", full),
                kind: RelationshipKind::Imports,
                alias: None,
                properties_accessed: Vec::new(),
                context: context.to_string(),
                file: file.to_string(),
                line,
            });
        }
        "identifier" | "scoped_identifier" | "self" => {
            let name = node_text(node, source).unwrap_or("");
            let full = if prefix.is_empty() { name.to_string() } else { format!("{}::{}", prefix, name) };
            out.push(Relationship {
                from: make_symbol_id(file, "module", &SymbolKind::Module),
                to: format!("unresolved_import::{}", full),
                kind: RelationshipKind::Imports,
                alias: None,
                properties_accessed: Vec::new(),
                context: context.to_string(),
                file: file.to_string(),
                line,
            });
        }
        _ => {}
    }
}

fn extract_impl_block(
    node: Node<'_>,
    source: &[u8],
    file: &str,
    lang: Language,
    symbols: &mut Vec<Symbol>,
) -> Vec<Relationship> {
    let mut out = Vec::new();
    let line = node.start_position().row as u32 + 1;

    let trait_node = node.child_by_field_name("trait");
    let type_node = node.child_by_field_name("type");

    if let (Some(trait_n), Some(type_n)) = (trait_node, type_node) {
        let trait_name = node_text(trait_n, source).unwrap_or("");
        let type_name = node_text(type_n, source).unwrap_or("");
        if !trait_name.is_empty() && !type_name.is_empty() {
            out.push(Relationship {
                from: make_symbol_id(file, type_name, &SymbolKind::Class),
                to: format!("unresolved_import::{}", trait_name),
                kind: RelationshipKind::Implements,
                alias: None,
                properties_accessed: Vec::new(),
                context: node_text(node, source).unwrap_or("").lines().next().unwrap_or("").to_string(),
                file: file.to_string(),
                line,
            });
        }
    }

    if let Some(body) = node.child_by_field_name("body") {
        let impl_type = type_node.and_then(|n| node_text(n, source)).unwrap_or("unknown");
        walk_tree(body, &mut |child| {
            if child.kind() == "function_item" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let method_name = node_text(name_node, source).unwrap_or("");
                    if !method_name.is_empty() {
                        symbols.push(Symbol {
                            id: make_symbol_id(file, method_name, &SymbolKind::Method),
                            name: method_name.to_string(),
                            kind: SymbolKind::Method,
                            language: lang.clone(),
                            file: file.to_string(),
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            signature: Some(format!("impl {} :: {}", impl_type, method_name)),
                        });
                    }
                }
            }
        });
    }

    out
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
