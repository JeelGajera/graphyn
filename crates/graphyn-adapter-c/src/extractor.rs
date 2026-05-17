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
        "struct_specifier" | "class_specifier" => {
            if let Some(sym) = extract_struct_or_class(
                node,
                src,
                &parsed.file,
                parsed.language.clone(),
                &mut relationships,
            ) {
                symbols.push(sym);
            }
        }
        "enum_specifier" => {
            if let Some(sym) = extract_enum(node, src, &parsed.file, parsed.language.clone()) {
                symbols.push(sym);
            }
        }
        "function_definition" => {
            if let Some(sym) = extract_function(node, src, &parsed.file, parsed.language.clone()) {
                symbols.push(sym);
            }
        }
        "declaration" | "type_definition" => {
            relationships.extend(extract_typedef_decl(node, src, &parsed.file))
        }
        "preproc_include" => {
            if let Some(rel) = extract_include(node, src, &parsed.file) {
                relationships.push(rel);
            }
        }
        "alias_declaration" => {
            if let Some(rel) = extract_using_alias(node, src, &parsed.file) {
                relationships.push(rel);
            }
        }
        "field_expression" => {
            if let (Some(arg), Some(field)) = (node.child_by_field_name("argument"), node.child_by_field_name("field")) {
                let obj = node_text(arg, src).unwrap_or("");
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

fn extract_struct_or_class(
    node: Node<'_>,
    source: &[u8],
    file: &str,
    lang: Language,
    relationships: &mut Vec<Relationship>,
) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    if name.is_empty() {
        return None;
    }

    if let Some(base_clause) = node.child_by_field_name("base_class_clause") {
        walk_tree(base_clause, &mut |bc| {
            if bc.kind() == "type_identifier" || bc.kind() == "qualified_identifier" {
                let base_name = node_text(bc, source).unwrap_or("");
                if !base_name.is_empty() {
                    relationships.push(Relationship {
                        from: make_symbol_id(file, name, &SymbolKind::Class),
                        to: format!("unresolved_alias::{}", base_name),
                        kind: RelationshipKind::Extends,
                        alias: None,
                        properties_accessed: Vec::new(),
                        context: node_text(node, source)
                            .unwrap_or("")
                            .lines()
                            .next()
                            .unwrap_or("")
                            .to_string(),
                        file: file.to_string(),
                        line: node.start_position().row as u32 + 1,
                    });
                }
            }
        });
    }

    Some(Symbol {
        id: make_symbol_id(file, name, &SymbolKind::Class),
        name: name.to_string(),
        kind: SymbolKind::Class,
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

fn extract_enum(node: Node<'_>, source: &[u8], file: &str, lang: Language) -> Option<Symbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source)?;
    Some(Symbol {
        id: make_symbol_id(file, name, &SymbolKind::Enum),
        name: name.to_string(),
        kind: SymbolKind::Enum,
        language: lang,
        file: file.to_string(),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        signature: Some(node_text(node, source).unwrap_or("").to_string()),
    })
}

fn extract_function(node: Node<'_>, source: &[u8], file: &str, lang: Language) -> Option<Symbol> {
    let declarator = node.child_by_field_name("declarator")?;
    let name = node_text(declarator, source)?;
    let fn_name = name.split('(').next().unwrap_or(name).trim();
    if fn_name.is_empty() {
        return None;
    }
    Some(Symbol {
        id: make_symbol_id(file, fn_name, &SymbolKind::Function),
        name: fn_name.to_string(),
        kind: SymbolKind::Function,
        language: lang,
        file: file.to_string(),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        signature: Some(node_text(node, source).unwrap_or("").lines().next().unwrap_or("").to_string()),
    })
}

fn extract_include(node: Node<'_>, source: &[u8], file: &str) -> Option<Relationship> {
    let text = node_text(node, source)?;
    Some(Relationship {
        from: make_symbol_id(file, "module", &SymbolKind::Module),
        to: format!("unresolved_include::{}", text),
        kind: RelationshipKind::Imports,
        alias: None,
        properties_accessed: Vec::new(),
        context: text.to_string(),
        file: file.to_string(),
        line: node.start_position().row as u32 + 1,
    })
}

fn extract_typedef_decl(node: Node<'_>, source: &[u8], file: &str) -> Vec<Relationship> {
    let mut out = Vec::new();
    let text = node_text(node, source).unwrap_or("");
    let text_compact = text.replace('\n', " ");
    if !text.contains("typedef") {
        return out;
    }

    let re = regex::Regex::new(r"typedef\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*;")
        .expect("valid regex");
    if let Some(caps) = re.captures(&text_compact) {
        let base = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let alias = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        if !base.is_empty() && !alias.is_empty() && base != alias {
            out.push(Relationship {
                from: make_symbol_id(file, "module", &SymbolKind::Module),
                to: format!("unresolved_alias::{}", base),
                kind: RelationshipKind::Imports,
                alias: Some(alias.to_string()),
                properties_accessed: Vec::new(),
                context: text.to_string(),
                file: file.to_string(),
                line: node.start_position().row as u32 + 1,
            });
        }
    }
    out
}

fn extract_using_alias(node: Node<'_>, source: &[u8], file: &str) -> Option<Relationship> {
    let alias_node = node.child_by_field_name("name")?;
    let type_node = node.child_by_field_name("type")?;

    let alias = node_text(alias_node, source)?;
    let base = node_text(type_node, source)?;
    if alias == base {
        return None;
    }
    let base_leaf = base.rsplit("::").next().unwrap_or(base);

    Some(Relationship {
        from: make_symbol_id(file, "module", &SymbolKind::Module),
        to: format!("unresolved_alias::{}", base_leaf),
        kind: RelationshipKind::Imports,
        alias: Some(alias.to_string()),
        properties_accessed: Vec::new(),
        context: node_text(node, source).unwrap_or("").to_string(),
        file: file.to_string(),
        line: node.start_position().row as u32 + 1,
    })
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
