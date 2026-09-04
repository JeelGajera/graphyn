//! Resolving `#include` directives and the type names they bring into scope.
//!
//! Two bugs lived here. Includes resolved to `local_header::<name>`, a string
//! that is neither a symbol id nor an `ext::` package, so
//! [`graphyn_core::graph::GraphynGraph::add_relationship`] dropped every one of
//! them — a C repository produced a graph with zero edges. And type names were
//! looked up in a repository-wide, first-wins map, so a `typedef struct Foo Bar`
//! bound to whichever `Foo` happened to be indexed first regardless of what the
//! file could actually see.
//!
//! Includes now resolve to the header's module symbol, and a type name is only
//! visible if the file defines it or includes a header that does — which is
//! what the C compiler does too.

use std::collections::{BTreeSet, HashMap, HashSet};

use graphyn_core::ir::{
    Diagnostic, DiagnosticCategory, DiagnosticLevel, RelationshipKind, RepoIR, SymbolKind,
};
use graphyn_core::symbol_id::{
    external_package_id, module_symbol_id, parse_unresolved_local_type_id,
};

const INCLUDE_PREFIX: &str = "unresolved_include";

/// Placeholder for an `#include`, remembering whether it was quoted or angled.
///
/// The distinction matters: `"foo.h"` is searched relative to the including
/// file and is almost always in-repo, while `<foo.h>` comes from an include
/// path we cannot see and is almost always a system or third-party header.
pub fn unresolved_include_id(path: &str, is_local: bool) -> String {
    let kind = if is_local { "local" } else { "system" };
    format!("{INCLUDE_PREFIX}|{kind}|{path}")
}

fn parse_unresolved_include_id(raw: &str) -> Option<(bool, &str)> {
    let rest = raw.strip_prefix(INCLUDE_PREFIX)?.strip_prefix('|')?;
    let (kind, path) = rest.split_once('|')?;
    Some((kind == "local", path))
}

pub fn resolve_repo_ir(_root: &std::path::Path, repo_ir: &mut RepoIR) {
    let known_files: HashSet<String> = repo_ir.files.iter().map(|f| f.file.clone()).collect();

    // Basename → files with that name, for includes that rely on an `-I` path
    // we cannot reconstruct. Only used when the name is unambiguous.
    let mut by_basename: HashMap<String, Vec<String>> = HashMap::new();
    for file in &repo_ir.files {
        by_basename
            .entry(basename(&file.file).to_string())
            .or_default()
            .push(file.file.clone());
    }

    // Symbols each file defines, for building the visible set below.
    let symbols_by_file: HashMap<String, HashMap<String, String>> = repo_ir
        .files
        .iter()
        .map(|file| {
            let symbols = file
                .symbols
                .iter()
                .filter(|s| s.kind != SymbolKind::Module)
                .map(|s| (s.name.clone(), s.id.clone()))
                .collect();
            (file.file.clone(), symbols)
        })
        .collect();

    // ── pass 1: includes ─────────────────────────────────────
    //
    // Done for every file up front, because pass 2 needs the include graph to
    // know which types each file can see.
    let mut includes: HashMap<String, Vec<String>> = HashMap::new();

    for file in &mut repo_ir.files {
        let path = file.file.clone();
        let mut resolved_includes = Vec::new();

        for rel in file.relationships.iter_mut() {
            let Some((is_local, include_path)) = parse_unresolved_include_id(&rel.to) else {
                continue;
            };
            let include_path = include_path.to_string();

            let target = if is_local {
                resolve_relative(&path, &include_path)
                    .filter(|candidate| known_files.contains(candidate))
                    .or_else(|| unique_basename_match(&by_basename, &include_path))
            } else {
                // An angled include may still be in-repo when the build passes
                // `-I`; accept it only on an unambiguous basename match.
                unique_basename_match(&by_basename, &include_path)
            };

            match target {
                Some(target) => {
                    rel.to = module_symbol_id(&target);
                    resolved_includes.push(target);
                }
                None => {
                    // Not in the repository: a system or third-party header.
                    // Recorded as an external package so the dependency is
                    // visible rather than dropped.
                    rel.to = external_package_id(&package_name_for(&include_path));
                }
            }
        }

        includes.insert(path, resolved_includes);
    }

    // ── pass 2: type names ───────────────────────────────────
    for index in 0..repo_ir.files.len() {
        let path = repo_ir.files[index].file.clone();

        // What this translation unit can see: its own symbols plus those of the
        // headers it includes, transitively — headers include headers.
        let visible = visible_symbols(&path, &includes, &symbols_by_file);

        // Aliases introduced by `typedef` / `using`, resolved first so later
        // references through the alias find the underlying type.
        let mut aliases: HashMap<String, String> = HashMap::new();
        let mut resolved_props: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut drop = BTreeSet::new();
        let mut diagnostics = Vec::new();

        // Aliases must be resolved before the accesses that use them, and the
        // extractor emits them in source order, so two sweeps are needed.
        for rel in &repo_ir.files[index].relationships.clone() {
            if rel.kind != RelationshipKind::Imports {
                continue;
            }
            let (Some(type_name), Some(alias)) = (
                parse_unresolved_local_type_id(&rel.to),
                rel.alias.as_deref(),
            ) else {
                continue;
            };
            if let Some(id) = visible.get(type_name) {
                aliases.insert(alias.to_string(), id.clone());
            }
        }

        let file = &mut repo_ir.files[index];
        for (position, rel) in file.relationships.iter_mut().enumerate() {
            let Some(type_name) = parse_unresolved_local_type_id(&rel.to).map(str::to_string)
            else {
                continue;
            };

            let resolved = visible
                .get(&type_name)
                .or_else(|| aliases.get(&type_name))
                .cloned();

            match resolved {
                Some(id) => {
                    if rel.kind == RelationshipKind::AccessesProperty {
                        resolved_props
                            .entry(id.clone())
                            .or_default()
                            .extend(rel.properties_accessed.iter().cloned());
                    }
                    rel.to = id;
                }
                None => {
                    drop.insert(position);
                    if !crate::lang::c::scope_analyzer::is_builtin_type(&type_name) {
                        diagnostics.push(Diagnostic {
                            level: DiagnosticLevel::Warning,
                            category: DiagnosticCategory::Resolution,
                            message: format!(
                                "unable to resolve type '{type_name}'; \
                                 no included header defines it"
                            ),
                            file: Some(path.clone()),
                            line: Some(rel.line),
                        });
                    }
                }
            }
        }

        // An alias import carries the members reached through the alias.
        for rel in file.relationships.iter_mut() {
            if rel.kind != RelationshipKind::Imports {
                continue;
            }
            if let Some(props) = resolved_props.get(&rel.to) {
                rel.properties_accessed = props.iter().cloned().collect();
            }
        }

        file.diagnostics.extend(diagnostics);

        if !drop.is_empty() {
            let mut position = 0usize;
            file.relationships.retain(|_| {
                let keep = !drop.contains(&position);
                position += 1;
                keep
            });
        }
    }
}

/// Symbols reachable from `file`: its own, plus every header it includes.
fn visible_symbols(
    file: &str,
    includes: &HashMap<String, Vec<String>>,
    symbols_by_file: &HashMap<String, HashMap<String, String>>,
) -> HashMap<String, String> {
    let mut visible = HashMap::new();
    let mut seen = HashSet::new();
    let mut queue = vec![file.to_string()];

    while let Some(current) = queue.pop() {
        if !seen.insert(current.clone()) {
            continue; // headers include each other; visit each once
        }
        if let Some(symbols) = symbols_by_file.get(&current) {
            for (name, id) in symbols {
                // The including file wins on a clash, matching C's shadowing.
                visible.entry(name.clone()).or_insert_with(|| id.clone());
            }
        }
        if let Some(headers) = includes.get(&current) {
            queue.extend(headers.iter().cloned());
        }
    }

    visible
}

/// Resolve `#include "../include/foo.h"` against the including file's directory.
fn resolve_relative(from_file: &str, include_path: &str) -> Option<String> {
    let mut parts: Vec<&str> = from_file.split('/').collect();
    parts.pop(); // drop the filename

    for segment in include_path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }

    Some(parts.join("/"))
}

/// Match on filename alone, but only when exactly one file could be meant.
fn unique_basename_match(
    by_basename: &HashMap<String, Vec<String>>,
    include_path: &str,
) -> Option<String> {
    let candidates = by_basename.get(basename(include_path))?;
    match candidates.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    }
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// A stable node name for a header outside the repository.
///
/// `<sys/socket.h>` becomes `sys/socket.h`, keeping system headers from the
/// same family grouped rather than collapsed into one opaque node.
fn package_name_for(include_path: &str) -> String {
    include_path.trim_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_includes_normalise_against_the_including_file() {
        assert_eq!(
            resolve_relative("src/view_model_mapper.c", "../include/user_payload.h").as_deref(),
            Some("include/user_payload.h")
        );
        assert_eq!(
            resolve_relative("src/a/b.c", "./local.h").as_deref(),
            Some("src/a/local.h")
        );
        assert_eq!(
            resolve_relative("main.c", "lib/util.h").as_deref(),
            Some("lib/util.h")
        );
    }

    #[test]
    fn includes_escaping_the_root_do_not_resolve() {
        assert_eq!(resolve_relative("main.c", "../../outside.h"), None);
    }

    #[test]
    fn include_placeholders_round_trip() {
        let local = unresolved_include_id("../include/a.h", true);
        assert_eq!(
            parse_unresolved_include_id(&local),
            Some((true, "../include/a.h"))
        );
        let system = unresolved_include_id("stdio.h", false);
        assert_eq!(
            parse_unresolved_include_id(&system),
            Some((false, "stdio.h"))
        );
    }
}
