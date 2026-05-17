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

fn module_symbol(file: &str) -> Symbol {
    Symbol {
        id: make_symbol_id(file, "module", &SymbolKind::Module),
        name: "module".to_string(),
        kind: SymbolKind::Module,
        language: Language::Go,
        file: file.to_string(),
        line_start: 1,
        line_end: 1,
        signature: None,
    }
}

pub fn extract_file_ir(parsed: &ParsedFile) -> FileIR {
    let src = parsed.source.as_bytes();
    let root = parsed.tree.root_node();

    let mut symbols = vec![module_symbol(&parsed.file)];
    let mut relationships = Vec::new();

    walk_tree(root, &mut |node| match node.kind() {
        "type_declaration" => {
            extract_type_declaration(node, src, &parsed.file, &mut symbols);
        }
        "function_declaration" => {
            if let Some(sym) = extract_function_decl(node, src, &parsed.file) {
                symbols.push(sym);
            }
        }
        "method_declaration" => {
            if let Some(sym) = extract_method_decl(node, src, &parsed.file) {
                symbols.push(sym);
            }
        }
        "import_declaration" => {
            relationships.extend(extract_import_decl(node, src, &parsed.file));
        }
        "selector_expression" => {
            if let (Some(op), Some(field)) = (node.child_by_field_name("operand"), node.child_by_field_name("field")) {
                let obj = node_text(op, src).unwrap_or("");
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
        language: Language::Go,
        symbols,
        relationships,
        diagnostics: Vec::new(),
        re_exports: Vec::new(),
    }
}

fn extract_type_declaration(node: Node<'_>, source: &[u8], file: &str, symbols: &mut Vec<Symbol>) {
    walk_tree(node, &mut |child| {
        if child.kind() == "type_spec" {
            let name = child
                .child_by_field_name("name")
                .and_then(|n| node_text(n, source))
                .unwrap_or("");
            if name.is_empty() {
                return;
            }
            let ty = child.child_by_field_name("type");
            let kind = match ty.map(|n| n.kind()) {
                Some("interface_type") => SymbolKind::Interface,
                _ => SymbolKind::Class,
            };
            symbols.push(Symbol {
                id: make_symbol_id(file, name, &kind),
                name: name.to_string(),
                kind,
                language: Language::Go,
                file: file.to_string(),
                line_start: child.start_position().row as u32 + 1,
                line_end: child.end_position().row as u32 + 1,
                signature: Some(node_text(child, source).unwrap_or("").to_string()),
            });
        }
    });
}

fn extract_function_decl(node: Node<'_>, source: &[u8], file: &str) -> Option<Symbol> {
    let name = node.child_by_field_name("name").and_then(|n| node_text(n, source))?;
    Some(Symbol {
        id: make_symbol_id(file, name, &SymbolKind::Function),
        name: name.to_string(),
        kind: SymbolKind::Function,
        language: Language::Go,
        file: file.to_string(),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        signature: Some(node_text(node, source).unwrap_or("").lines().next().unwrap_or("").to_string()),
    })
}

fn extract_method_decl(node: Node<'_>, source: &[u8], file: &str) -> Option<Symbol> {
    let name = node.child_by_field_name("name").and_then(|n| node_text(n, source))?;
    Some(Symbol {
        id: make_symbol_id(file, name, &SymbolKind::Method),
        name: name.to_string(),
        kind: SymbolKind::Method,
        language: Language::Go,
        file: file.to_string(),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        signature: Some(node_text(node, source).unwrap_or("").lines().next().unwrap_or("").to_string()),
    })
}

fn extract_import_decl(node: Node<'_>, source: &[u8], file: &str) -> Vec<Relationship> {
    let mut out = Vec::new();
    let line = node.start_position().row as u32 + 1;

    walk_tree(node, &mut |child| {
        if child.kind() != "import_spec" {
            return;
        }

        let path_node = child.child_by_field_name("path");
        let name_node = child.child_by_field_name("name");

        if let Some(path_n) = path_node {
            let raw_path = node_text(path_n, source).unwrap_or("");
            let pkg_path = raw_path.trim_matches('"');
            let import_kind = name_node.and_then(|n| node_text(n, source)).map(|s| s.to_string());
            let effective_alias = match import_kind.as_deref() {
                Some("_") => None,
                Some(".") => None,
                Some(name) => Some(name.to_string()),
                None => None,
            };

            out.push(Relationship {
                from: make_symbol_id(file, "module", &SymbolKind::Module),
                to: format!("unresolved_import::{}", pkg_path),
                kind: RelationshipKind::Imports,
                alias: effective_alias,
                properties_accessed: Vec::new(),
                context: node_text(child, source).unwrap_or("").to_string(),
                file: file.to_string(),
                line,
            });
        }
    });

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
