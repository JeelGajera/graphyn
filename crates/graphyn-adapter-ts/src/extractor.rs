use std::collections::{BTreeMap, BTreeSet, HashMap};

use graphyn_core::ir::{FileIR, Language, Relationship, RelationshipKind, Symbol, SymbolKind};

use crate::parser::ParsedFile;

const UNRESOLVED_IMPORT_PREFIX: &str = "__UNRESOLVED_IMPORT__";
const UNRESOLVED_LOCAL_TYPE_PREFIX: &str = "__UNRESOLVED_LOCAL_TYPE__";

pub fn extract_file_ir(parsed: &ParsedFile) -> FileIR {
    let mut symbols = extract_symbols(&parsed.file, parsed.language.clone(), &parsed.source);
    let module_symbol = module_symbol(&parsed.file, parsed.language.clone());
    if !symbols.iter().any(|s| s.id == module_symbol.id) {
        symbols.push(module_symbol.clone());
    }

    let from_symbol_id = first_primary_symbol_id(&symbols).unwrap_or(module_symbol.id.clone());

    let import_entries = parse_import_and_reexport_lines(&parsed.source);
    let var_to_type = parse_typed_variables(&parsed.source);
    let type_to_props = parse_property_accesses(&parsed.source, &var_to_type);

    let mut relationships = Vec::new();

    for import in import_entries {
        let mut properties_accessed = Vec::new();
        if import.kind == RelationshipKind::Imports {
            properties_accessed = type_to_props
                .get(&import.local_name)
                .cloned()
                .unwrap_or_default();
        }

        relationships.push(Relationship {
            from: from_symbol_id.clone(),
            to: unresolved_import_symbol_id(&import.module_specifier, &import.imported_name),
            kind: import.kind,
            alias: if import.local_name != import.imported_name {
                Some(import.local_name)
            } else {
                None
            },
            properties_accessed,
            context: import.context,
            file: parsed.file.clone(),
            line: import.line,
        });
    }

    for (type_name, properties) in type_to_props {
        if properties.is_empty() {
            continue;
        }
        relationships.push(Relationship {
            from: from_symbol_id.clone(),
            to: unresolved_local_type_symbol_id(&type_name),
            kind: RelationshipKind::AccessesProperty,
            alias: Some(type_name),
            properties_accessed: properties,
            context: "property access".to_string(),
            file: parsed.file.clone(),
            line: 1,
        });
    }

    FileIR {
        file: parsed.file.clone(),
        language: parsed.language.clone(),
        symbols,
        relationships,
        parse_errors: parsed.parse_errors.clone(),
    }
}

pub fn unresolved_import_symbol_id(module_specifier: &str, symbol_name: &str) -> String {
    format!("{UNRESOLVED_IMPORT_PREFIX}|{module_specifier}|{symbol_name}")
}

pub fn unresolved_local_type_symbol_id(type_name: &str) -> String {
    format!("{UNRESOLVED_LOCAL_TYPE_PREFIX}|{type_name}")
}

pub fn parse_unresolved_import_symbol_id(raw: &str) -> Option<(String, String)> {
    let mut parts = raw.splitn(3, '|');
    let prefix = parts.next()?;
    if prefix != UNRESOLVED_IMPORT_PREFIX {
        return None;
    }
    let module = parts.next()?.to_string();
    let symbol = parts.next()?.to_string();
    Some((module, symbol))
}

pub fn parse_unresolved_local_type_symbol_id(raw: &str) -> Option<String> {
    let mut parts = raw.splitn(2, '|');
    let prefix = parts.next()?;
    if prefix != UNRESOLVED_LOCAL_TYPE_PREFIX {
        return None;
    }
    Some(parts.next()?.to_string())
}

#[derive(Debug, Clone)]
struct ImportEntry {
    imported_name: String,
    local_name: String,
    module_specifier: String,
    kind: RelationshipKind,
    context: String,
    line: u32,
}

fn parse_import_and_reexport_lines(source: &str) -> Vec<ImportEntry> {
    let mut out = Vec::new();

    for (idx, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.starts_with("import ") && line.contains(" from ") {
            if let Some(module_specifier) = parse_module_specifier(line) {
                if line.starts_with("import {")
                    || line.starts_with("import {")
                    || line.contains(" import {")
                {
                    if let Some(named_block) = between(line, '{', '}') {
                        for item in named_block
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                        {
                            let (imported_name, local_name) = parse_aliased_item(item);
                            out.push(ImportEntry {
                                imported_name,
                                local_name,
                                module_specifier: module_specifier.clone(),
                                kind: RelationshipKind::Imports,
                                context: line.to_string(),
                                line: (idx + 1) as u32,
                            });
                        }
                    }
                } else if let Some((left, _)) = line.split_once(" from ") {
                    let default = left.trim_start_matches("import").trim();
                    if !default.is_empty() {
                        // Handles: import User from './x'
                        let local_name = default
                            .split(',')
                            .next()
                            .unwrap_or(default)
                            .trim()
                            .to_string();
                        if !local_name.is_empty() {
                            out.push(ImportEntry {
                                imported_name: "default".to_string(),
                                local_name,
                                module_specifier,
                                kind: RelationshipKind::Imports,
                                context: line.to_string(),
                                line: (idx + 1) as u32,
                            });
                        }
                    }
                }
            }
        }

        if line.starts_with("export ") && line.contains(" from ") {
            if let Some(module_specifier) = parse_module_specifier(line) {
                if line.starts_with("export {") {
                    if let Some(named_block) = between(line, '{', '}') {
                        for item in named_block
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                        {
                            let (imported_name, local_name) = parse_aliased_item(item);
                            out.push(ImportEntry {
                                imported_name,
                                local_name,
                                module_specifier: module_specifier.clone(),
                                kind: RelationshipKind::ReExports,
                                context: line.to_string(),
                                line: (idx + 1) as u32,
                            });
                        }
                    }
                } else if line.starts_with("export *") {
                    out.push(ImportEntry {
                        imported_name: "*".to_string(),
                        local_name: "*".to_string(),
                        module_specifier,
                        kind: RelationshipKind::ReExports,
                        context: line.to_string(),
                        line: (idx + 1) as u32,
                    });
                }
            }
        }
    }

    out
}

fn parse_typed_variables(source: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();

    for line in source.lines() {
        let mut rest = line;
        while let Some(open_idx) = rest.find('(') {
            let after_open = &rest[open_idx + 1..];
            let Some(close_idx) = after_open.find(')') else {
                break;
            };
            let args = &after_open[..close_idx];
            for arg in args.split(',') {
                if let Some((name, typ)) = arg.split_once(':') {
                    let name = sanitize_identifier(name);
                    let typ = sanitize_type_identifier(typ);
                    if !name.is_empty() && !typ.is_empty() {
                        out.insert(name.to_string(), typ.to_string());
                    }
                }
            }
            rest = &after_open[close_idx + 1..];
        }
    }

    out
}

fn parse_property_accesses(
    source: &str,
    var_to_type: &HashMap<String, String>,
) -> BTreeMap<String, Vec<String>> {
    let mut type_to_properties: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for line in source.lines() {
        for (var, type_name) in var_to_type {
            let needle = format!("{var}.");
            let mut rest = line;
            while let Some(idx) = rest.find(&needle) {
                let after = &rest[idx + needle.len()..];
                let property = read_identifier(after);
                if !property.is_empty() {
                    type_to_properties
                        .entry(type_name.clone())
                        .or_default()
                        .insert(property.to_string());
                }
                rest = after;
            }
        }
    }

    type_to_properties
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
}

fn extract_symbols(file: &str, language: Language, source: &str) -> Vec<Symbol> {
    let mut out = Vec::new();

    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        let line = raw_line.trim();

        if let Some(name) = read_keyword_name(line, "class") {
            out.push(new_symbol(
                file,
                &name,
                SymbolKind::Class,
                language.clone(),
                line_no,
                line,
            ));
        }
        if let Some(name) = read_keyword_name(line, "interface") {
            out.push(new_symbol(
                file,
                &name,
                SymbolKind::Interface,
                language.clone(),
                line_no,
                line,
            ));
        }
        if let Some(name) = read_keyword_name(line, "type") {
            out.push(new_symbol(
                file,
                &name,
                SymbolKind::TypeAlias,
                language.clone(),
                line_no,
                line,
            ));
        }
        if let Some(name) = read_keyword_name(line, "function") {
            out.push(new_symbol(
                file,
                &name,
                SymbolKind::Function,
                language.clone(),
                line_no,
                line,
            ));
        }
        if let Some(name) = read_keyword_name(line, "enum") {
            out.push(new_symbol(
                file,
                &name,
                SymbolKind::Enum,
                language.clone(),
                line_no,
                line,
            ));
        }

        if is_property_declaration(line) {
            if let Some((name, _)) = line.split_once(':') {
                let prop_name = sanitize_identifier(name);
                if !prop_name.is_empty() {
                    out.push(new_symbol(
                        file,
                        prop_name,
                        SymbolKind::Property,
                        language.clone(),
                        line_no,
                        line,
                    ));
                }
            }
        }
    }

    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);
    out
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
        signature: Some("module".to_string()),
    }
}

fn first_primary_symbol_id(symbols: &[Symbol]) -> Option<String> {
    symbols
        .iter()
        .find(|s| s.kind != SymbolKind::Module)
        .map(|s| s.id.clone())
}

fn new_symbol(
    file: &str,
    name: &str,
    kind: SymbolKind,
    language: Language,
    line: u32,
    signature: &str,
) -> Symbol {
    Symbol {
        id: make_symbol_id(file, name, &kind),
        name: name.to_string(),
        kind,
        language,
        file: file.to_string(),
        line_start: line,
        line_end: line,
        signature: Some(signature.to_string()),
    }
}

fn make_symbol_id(file: &str, name: &str, kind: &SymbolKind) -> String {
    format!("{file}::{name}::{}", symbol_kind_suffix(kind))
}

fn symbol_kind_suffix(kind: &SymbolKind) -> &'static str {
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
    }
}

fn parse_module_specifier(line: &str) -> Option<String> {
    let quote_start = line.find('\'').or_else(|| line.find('"'))?;
    let quote_char = line.chars().nth(quote_start)?;
    let rest = &line[quote_start + 1..];
    let end = rest.find(quote_char)?;
    Some(rest[..end].to_string())
}

fn parse_aliased_item(item: &str) -> (String, String) {
    if let Some((left, right)) = item.split_once(" as ") {
        (
            sanitize_identifier(left).to_string(),
            sanitize_identifier(right).to_string(),
        )
    } else {
        let cleaned = sanitize_identifier(item).to_string();
        (cleaned.clone(), cleaned)
    }
}

fn between(input: &str, open: char, close: char) -> Option<String> {
    let start = input.find(open)?;
    let end = input.rfind(close)?;
    if end <= start {
        return None;
    }
    Some(input[start + 1..end].to_string())
}

fn sanitize_identifier(input: &str) -> &str {
    input
        .trim()
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '$')
}

fn sanitize_type_identifier(input: &str) -> &str {
    let raw = sanitize_identifier(input);
    raw.split(['<', '>', '|', '[', ']'])
        .next()
        .unwrap_or(raw)
        .trim()
}

fn read_keyword_name(line: &str, keyword: &str) -> Option<String> {
    let needle = format!("{keyword} ");
    let idx = line.find(&needle)?;
    let tail = &line[idx + needle.len()..];
    let ident = read_identifier(tail);
    if ident.is_empty() {
        None
    } else {
        Some(ident.to_string())
    }
}

fn read_identifier(input: &str) -> &str {
    let end = input
        .char_indices()
        .find(|(_, c)| !c.is_ascii_alphanumeric() && *c != '_' && *c != '$')
        .map(|(idx, _)| idx)
        .unwrap_or(input.len());
    &input[..end]
}

fn is_property_declaration(line: &str) -> bool {
    line.ends_with(';')
        && line.contains(':')
        && !line.contains("import ")
        && !line.contains("export ")
        && !line.contains('(')
        && !line.contains("=>")
}
