use graphyn_core::ast::{first_line_of, node_text, start_line, walk};
use graphyn_core::ir::{
    Diagnostic, DiagnosticCategory, DiagnosticLevel, FileIR, Language, Relationship,
    RelationshipKind, Symbol, SymbolKind,
};
use graphyn_core::symbol_id::{
    make_symbol_id, module_symbol, module_symbol_id, unresolved_import_id, unresolved_local_type_id,
    IMPORT_ALL,
};
use tree_sitter::Node;

use crate::parser::ParsedFile;
use crate::scope_analyzer::{analyze, is_builtin_type};

pub fn extract_file_ir(parsed: &ParsedFile) -> FileIR {
    let source = parsed.source.as_bytes();
    let root = parsed.tree.root_node();
    let file = parsed.file.as_str();

    let mut symbols = vec![module_symbol(file, Language::Go)];
    let mut relationships = Vec::new();
    let mut diagnostics = parsed.diagnostics.clone();

    let stats = walk(root, &mut |node| match node.kind() {
        "type_declaration" => type_declaration(node, source, file, &mut symbols),
        "function_declaration" => {
            if let Some(name) = field_name(node, source) {
                symbols.push(symbol(file, name, name, SymbolKind::Function, node, source));
            }
        }
        "method_declaration" => method_declaration(node, source, file, &mut symbols),
        "import_declaration" => relationships.extend(imports(node, source, file)),
        _ => {}
    });

    // Property accesses and type references, both grounded in declared types.
    let analysis = analyze(root, source);

    for (type_name, access) in analysis.accesses {
        if access.properties.is_empty() {
            continue;
        }
        relationships.push(Relationship {
            from: module_symbol_id(file),
            to: unresolved_local_type_id(&type_name),
            kind: RelationshipKind::AccessesProperty,
            alias: Some(type_name),
            properties_accessed: access.properties.into_iter().collect(),
            context: "property access".to_string(),
            file: file.to_string(),
            line: access.first_line,
        });
    }

    for (type_name, line) in analysis.referenced_types {
        if is_builtin_type(&type_name) {
            continue;
        }
        relationships.push(Relationship {
            from: module_symbol_id(file),
            to: unresolved_local_type_id(&type_name),
            kind: RelationshipKind::UsesType,
            alias: Some(type_name),
            properties_accessed: Vec::new(),
            context: "type reference".to_string(),
            file: file.to_string(),
            line,
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
        language: Language::Go,
        symbols,
        relationships,
        diagnostics,
        re_exports: Vec::new(),
    }
}

fn field_name<'a>(node: Node<'_>, source: &'a [u8]) -> Option<&'a str> {
    node.child_by_field_name("name")
        .and_then(|n| node_text(n, source))
}

fn symbol(
    file: &str,
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
        language: Language::Go,
        file: file.to_string(),
        line_start: start_line(node),
        line_end: node.end_position().row as u32 + 1,
        signature: first_line_of(node, source),
    }
}

fn type_declaration(node: Node<'_>, source: &[u8], file: &str, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();
    for spec in node.children(&mut cursor) {
        if !matches!(spec.kind(), "type_spec" | "type_alias") {
            continue;
        }
        let Some(name) = field_name(spec, source) else {
            continue;
        };
        let body = spec.child_by_field_name("type");
        let kind = match (spec.kind(), body.map(|n| n.kind())) {
            (_, Some("interface_type")) => SymbolKind::Interface,
            ("type_alias", _) => SymbolKind::TypeAlias,
            _ => SymbolKind::Class,
        };

        let is_interface = kind == SymbolKind::Interface;
        symbols.push(symbol(file, name, name, kind, spec, source));

        // Interface methods become symbols owned by the interface, which is how
        // `interface_detector` later learns the method set it requires without
        // re-parsing signature text.
        if is_interface {
            if let Some(body) = body {
                symbols.extend(interface_methods(body, source, file, name));
            }
        }
    }
}

fn interface_methods(
    body: Node<'_>,
    source: &[u8],
    file: &str,
    interface: &str,
) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut cursor = body.walk();
    for member in body.children(&mut cursor) {
        if member.kind() != "method_elem" && member.kind() != "method_spec" {
            continue;
        }
        let Some(name) = field_name(member, source) else {
            continue;
        };
        out.push(symbol(
            file,
            &format!("{interface}::{name}"),
            name,
            SymbolKind::Method,
            member,
            source,
        ));
    }
    out
}

fn method_declaration(node: Node<'_>, source: &[u8], file: &str, symbols: &mut Vec<Symbol>) {
    let Some(name) = field_name(node, source) else {
        return;
    };
    // The receiver type owns the method; qualifying the id keeps two `Map`
    // methods on different types from colliding on one node.
    let owner = node
        .child_by_field_name("receiver")
        .and_then(|r| receiver_type(r, source))
        .unwrap_or_else(|| "func".to_string());

    symbols.push(symbol(
        file,
        &format!("{owner}::{name}"),
        name,
        SymbolKind::Method,
        node,
        source,
    ));
}

fn receiver_type(receiver: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = receiver.walk();
    for param in receiver.children(&mut cursor) {
        if param.kind() != "parameter_declaration" {
            continue;
        }
        let ty = param.child_by_field_name("type")?;
        return bare_type_name(ty, source);
    }
    None
}

fn bare_type_name(ty: Node<'_>, source: &[u8]) -> Option<String> {
    match ty.kind() {
        "type_identifier" => node_text(ty, source).map(str::to_string),
        "pointer_type" | "generic_type" => ty
            .named_child(0)
            .and_then(|inner| bare_type_name(inner, source)),
        _ => None,
    }
}

/// One relationship per `import` spec.
///
/// Go imports a package, never a symbol, so the placeholder always names the
/// whole package. Which member is used is recorded separately, by the qualified
/// references the scope analyzer collects.
fn imports(node: Node<'_>, source: &[u8], file: &str) -> Vec<Relationship> {
    let mut out = Vec::new();

    walk(node, &mut |spec| {
        if spec.kind() != "import_spec" {
            return;
        }
        let Some(raw_path) = spec
            .child_by_field_name("path")
            .and_then(|p| node_text(p, source))
        else {
            return;
        };
        let package_path = raw_path.trim_matches('"');

        // `_ "pkg"` is a blank import for side effects and `. "pkg"` dumps the
        // package into file scope; neither introduces a usable local name.
        let alias = match spec
            .child_by_field_name("name")
            .and_then(|n| node_text(n, source))
        {
            Some("_") | Some(".") | None => None,
            Some(name) => Some(name.to_string()),
        };

        out.push(Relationship {
            from: module_symbol_id(file),
            to: unresolved_import_id(package_path, IMPORT_ALL),
            kind: RelationshipKind::Imports,
            alias,
            properties_accessed: Vec::new(),
            context: node_text(spec, source).unwrap_or_default().to_string(),
            file: file.to_string(),
            line: start_line(spec),
        });
    });

    out
}
