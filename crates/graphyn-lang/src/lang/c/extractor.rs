use graphyn_core::ast::{first_line_of, node_text, start_line, walk};
use graphyn_core::ir::{
    Diagnostic, DiagnosticCategory, DiagnosticLevel, FileIR, Language, Relationship,
    RelationshipKind, Symbol, SymbolKind,
};
use graphyn_core::symbol_id::{
    make_symbol_id, module_symbol, module_symbol_id, unresolved_local_type_id,
};
use tree_sitter::Node;

use crate::lang::c::include_resolver::unresolved_include_id;
use crate::lang::c::parser::ParsedFile;
use crate::lang::c::scope_analyzer::{
    collect_type_accesses, is_builtin_type, type_name_of, typedef_alias_names,
};

pub fn extract_file_ir(parsed: &ParsedFile) -> FileIR {
    let source = parsed.source.as_bytes();
    let root = parsed.tree.root_node();
    let file = parsed.file.as_str();
    let language = parsed.language.clone();

    let mut symbols = vec![module_symbol(file, language.clone())];
    let mut relationships = Vec::new();
    let mut diagnostics = parsed.diagnostics.clone();

    let stats = walk(root, &mut |node| match node.kind() {
        "struct_specifier" | "union_specifier" | "class_specifier" => record_record_type(
            node,
            source,
            file,
            &language,
            &mut symbols,
            &mut relationships,
        ),
        "enum_specifier" => {
            // Same rule as records: only a definition has a body.
            if node.child_by_field_name("body").is_some() {
                if let Some(name) = named(node, source) {
                    symbols.push(symbol(
                        file,
                        &language,
                        name,
                        name,
                        SymbolKind::Enum,
                        node,
                        source,
                    ));
                }
            }
        }
        "function_definition" => {
            if let Some(name) = function_name(node, source) {
                symbols.push(symbol(
                    file,
                    &language,
                    &name,
                    &name,
                    SymbolKind::Function,
                    node,
                    source,
                ));
            }
        }
        "type_definition" => relationships.extend(typedef(node, source, file)),
        "alias_declaration" => relationships.extend(using_alias(node, source, file)),
        "preproc_include" => {
            if let Some(rel) = include(node, source, file) {
                relationships.push(rel);
            }
        }
        _ => {}
    });

    for (type_name, access) in collect_type_accesses(root, source) {
        if access.properties.is_empty() {
            continue;
        }
        relationships.push(Relationship {
            from: module_symbol_id(file),
            to: unresolved_local_type_id(&type_name),
            kind: RelationshipKind::AccessesProperty,
            alias: Some(type_name),
            properties_accessed: access.properties.into_iter().collect(),
            context: "member access".to_string(),
            file: file.to_string(),
            line: access.first_line,
        });
    }

    if stats.truncated() {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            category: DiagnosticCategory::Parse,
            message: format!(
                "{} deeply nested subtree(s) exceeded the traversal depth limit; \
                 symbols below them were not extracted",
                stats.skipped_subtrees
            ),
            file: Some(parsed.file.clone()),
            line: None,
        });
    }

    FileIR {
        file: parsed.file.clone(),
        language,
        symbols,
        relationships,
        diagnostics,
        re_exports: Vec::new(),
    }
}

/// Record a struct, union or class — but only where it is *defined*.
///
/// In C, `struct Foo *p` in a parameter list is also a `struct_specifier`.
/// Treating those as definitions minted one `Foo` symbol per file that merely
/// mentioned the type, which split the graph and made every struct name
/// ambiguous to `find_symbol_id`. A definition is the form that carries a body;
/// everything else is a reference.
fn record_record_type(
    node: Node<'_>,
    source: &[u8],
    file: &str,
    language: &Language,
    symbols: &mut Vec<Symbol>,
    relationships: &mut Vec<Relationship>,
) {
    let Some(name) = named(node, source) else {
        return;
    };

    if node.child_by_field_name("body").is_none() {
        // A forward declaration or a use of the type. Only record a reference
        // when it stands alone as a declaration, so we do not emit one edge per
        // parameter mention.
        if node.parent().map(|p| p.kind()) == Some("declaration") && !is_builtin_type(name) {
            relationships.push(Relationship {
                from: module_symbol_id(file),
                to: unresolved_local_type_id(name),
                kind: RelationshipKind::UsesType,
                alias: Some(name.to_string()),
                properties_accessed: Vec::new(),
                context: first_line_of(node, source).unwrap_or_default(),
                file: file.to_string(),
                line: start_line(node),
            });
        }
        return;
    }

    symbols.push(symbol(
        file,
        language,
        name,
        name,
        SymbolKind::Class,
        node,
        source,
    ));

    // C++ inheritance. The grammar makes `base_class_clause` an ordinary child
    // rather than a named field, so it is located by kind.
    let mut class_cursor = node.walk();
    let base_clause = node
        .children(&mut class_cursor)
        .find(|child| child.kind() == "base_class_clause");

    if let Some(bases) = base_clause {
        let mut cursor = bases.walk();
        for base in bases.children(&mut cursor) {
            // `: public Shape` — skip the colon and the access specifier.
            if matches!(base.kind(), ":" | "," | "access_specifier" | "virtual") {
                continue;
            }
            let Some(base_name) = type_name_of(base, source).or_else(|| {
                matches!(base.kind(), "type_identifier" | "qualified_identifier")
                    .then(|| node_text(base, source).map(str::to_string))
                    .flatten()
            }) else {
                continue;
            };
            relationships.push(Relationship {
                from: make_symbol_id(file, name, &SymbolKind::Class),
                to: unresolved_local_type_id(&base_name),
                kind: RelationshipKind::Extends,
                alias: Some(base_name),
                properties_accessed: Vec::new(),
                context: first_line_of(node, source).unwrap_or_default(),
                file: file.to_string(),
                line: start_line(node),
            });
        }
    }
}

fn named<'a>(node: Node<'_>, source: &'a [u8]) -> Option<&'a str> {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| node_text(n, source))?;
    (!name.is_empty()).then_some(name)
}

fn symbol(
    file: &str,
    language: &Language,
    id_name: &str,
    display_name: &str,
    kind: SymbolKind,
    node: Node<'_>,
    source: &[u8],
) -> Symbol {
    Symbol {
        id: make_symbol_id(file, id_name, &kind),
        name: display_name.to_string(),
        kind,
        language: language.clone(),
        file: file.to_string(),
        line_start: start_line(node),
        line_end: node.end_position().row as u32 + 1,
        signature: first_line_of(node, source),
    }
}

/// The declared name of a function, unwrapping pointers and qualifiers.
fn function_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    fn inner(node: Node<'_>, source: &[u8]) -> Option<String> {
        match node.kind() {
            "identifier" | "field_identifier" | "operator_name" | "destructor_name" => {
                node_text(node, source).map(str::to_string)
            }
            // `Class::method` — keep the qualification so two classes in one
            // file do not collide on a shared method name.
            "qualified_identifier" => node_text(node, source).map(str::to_string),
            _ => {
                let declarator = node
                    .child_by_field_name("declarator")
                    .or_else(|| node.named_child(0))?;
                inner(declarator, source)
            }
        }
    }
    inner(node.child_by_field_name("declarator")?, source)
}

/// `typedef struct Foo Bar;` — `Bar` is another name for `Foo`.
fn typedef(node: Node<'_>, source: &[u8], file: &str) -> Vec<Relationship> {
    let Some(target) = node
        .child_by_field_name("type")
        .and_then(|t| type_name_of(t, source))
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for alias in typedef_alias_names(node, source) {
        // `typedef struct Foo Foo;` is the idiomatic C self-naming typedef and
        // carries no relationship.
        if alias == target {
            continue;
        }
        out.push(Relationship {
            from: module_symbol_id(file),
            to: unresolved_local_type_id(&target),
            kind: RelationshipKind::Imports,
            alias: Some(alias),
            properties_accessed: Vec::new(),
            context: first_line_of(node, source).unwrap_or_default(),
            file: file.to_string(),
            line: start_line(node),
        });
    }
    out
}

/// C++ `using Bar = Foo;`
fn using_alias(node: Node<'_>, source: &[u8], file: &str) -> Vec<Relationship> {
    let (Some(alias), Some(target)) = (
        node.child_by_field_name("name")
            .and_then(|n| node_text(n, source)),
        node.child_by_field_name("type")
            .and_then(|t| type_name_of(t, source)),
    ) else {
        return Vec::new();
    };
    if alias == target {
        return Vec::new();
    }

    vec![Relationship {
        from: module_symbol_id(file),
        to: unresolved_local_type_id(&target),
        kind: RelationshipKind::Imports,
        alias: Some(alias.to_string()),
        properties_accessed: Vec::new(),
        context: first_line_of(node, source).unwrap_or_default(),
        file: file.to_string(),
        line: start_line(node),
    }]
}

fn include(node: Node<'_>, source: &[u8], file: &str) -> Option<Relationship> {
    let path_node = node.child_by_field_name("path")?;
    let raw = node_text(path_node, source)?;
    // `"local.h"` versus `<system.h>` — the quoting says where to look.
    let is_local = raw.starts_with('"');
    let path = raw.trim_matches(['"', '<', '>'].as_ref());

    Some(Relationship {
        from: module_symbol_id(file),
        to: unresolved_include_id(path, is_local),
        kind: RelationshipKind::Imports,
        alias: None,
        properties_accessed: Vec::new(),
        context: node_text(node, source)
            .unwrap_or_default()
            .trim()
            .to_string(),
        file: file.to_string(),
        line: start_line(node),
    })
}
