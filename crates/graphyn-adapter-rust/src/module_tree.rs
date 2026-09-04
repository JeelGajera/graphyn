//! Mapping Rust module paths to the files and symbols they name.
//!
//! The previous implementation resolved a `use` path by looking up its final
//! segment in a repository-wide map of every symbol name. That made
//! `use std::fmt::Debug` resolve to a local `Debug` if one existed anywhere,
//! and made two same-named types in different modules resolve to whichever was
//! indexed first. Resolution here is path-directed: a name is only found in the
//! module that actually declares or re-exports it.

use std::collections::HashMap;

use graphyn_core::ir::{FileIR, RelationshipKind, SymbolKind};
use graphyn_core::symbol_id::{module_symbol_id, parse_unresolved_import_id};

/// How far a `pub use` chain is followed before giving up.
///
/// Real re-export chains are two or three deep (`lib.rs` → `mod.rs` → the
/// defining module); the bound stops a cyclic `pub use` from looping forever.
const MAX_REEXPORT_DEPTH: usize = 8;

/// What a `use` path pointed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// A symbol defined in this repository.
    Symbol(String),
    /// A module in this repository, imported whole (`use foo::*`, `use foo;`).
    Module(String),
    /// A third-party crate, named by its first path segment.
    External(String),
    /// A local module path that exists, but does not export this name.
    UnknownMember,
    /// Nothing in the repository matches, and it does not look external either.
    Unknown,
}

#[derive(Debug, Default)]
pub struct ModuleTree {
    /// `crate::models::user_payload` → `src/models/user_payload.rs`
    module_to_file: HashMap<String, String>,
    /// file → symbol name → symbol id
    symbols_by_file: HashMap<String, HashMap<String, String>>,
    /// file → local name → the `(module, symbol)` it was imported from.
    /// Used to follow `pub use` re-export chains.
    imports_by_file: HashMap<String, HashMap<String, (String, String)>>,
    /// Top-level module names, used to tell `models::Foo` from `serde::Foo`.
    crate_roots: HashMap<String, ()>,
}

impl ModuleTree {
    pub fn build(files: &[FileIR]) -> Self {
        let mut tree = Self::default();

        for file in files {
            let module = file_to_module_path(&file.file);

            let symbols: HashMap<String, String> = file
                .symbols
                .iter()
                .filter(|s| s.kind != SymbolKind::Module)
                .map(|s| (s.name.clone(), s.id.clone()))
                .collect();

            let imports: HashMap<String, (String, String)> = file
                .relationships
                .iter()
                .filter(|r| r.kind == RelationshipKind::Imports)
                .filter_map(|r| {
                    let (module, symbol) = parse_unresolved_import_id(&r.to)?;
                    let local = r.alias.clone().unwrap_or_else(|| symbol.to_string());
                    Some((local, (module.to_string(), symbol.to_string())))
                })
                .collect();

            if let Some(root) = module
                .strip_prefix("crate::")
                .and_then(|m| m.split("::").next())
            {
                tree.crate_roots.insert(root.to_string(), ());
            }

            tree.module_to_file.insert(module, file.file.clone());
            tree.symbols_by_file.insert(file.file.clone(), symbols);
            tree.imports_by_file.insert(file.file.clone(), imports);
        }

        tree
    }

    /// Resolve `module::symbol` as written inside `current_file`.
    pub fn resolve(&self, current_file: &str, module: &str, symbol: &str) -> Resolved {
        let absolute = self.absolutize(current_file, module);

        let Some(target_file) = self.module_to_file.get(&absolute) else {
            return if self.looks_local(&absolute) {
                Resolved::Unknown
            } else {
                Resolved::External(first_segment(&absolute).to_string())
            };
        };

        if symbol == graphyn_core::symbol_id::IMPORT_ALL {
            return Resolved::Module(module_symbol_id(target_file));
        }

        match self.lookup_in_file(target_file, symbol, 0) {
            Some(id) => Resolved::Symbol(id),
            None => Resolved::UnknownMember,
        }
    }

    /// Find `symbol` in `file`, following `pub use` re-exports outward.
    fn lookup_in_file(&self, file: &str, symbol: &str, depth: usize) -> Option<String> {
        if let Some(id) = self.symbols_by_file.get(file).and_then(|s| s.get(symbol)) {
            return Some(id.clone());
        }
        if depth >= MAX_REEXPORT_DEPTH {
            return None;
        }

        // The module does not define it — it may re-export it.
        let (module, inner) = self.imports_by_file.get(file)?.get(symbol)?;
        let absolute = self.absolutize(file, module);
        let target = self.module_to_file.get(&absolute)?;
        if target == file {
            return None; // self-referential `pub use`, stop rather than loop
        }
        self.lookup_in_file(target, inner, depth + 1)
    }

    /// Expand `crate::`, `self::` and `super::` into an absolute module path.
    fn absolutize(&self, current_file: &str, module: &str) -> String {
        let here = file_to_module_path(current_file);

        if let Some(rest) = module.strip_prefix("crate") {
            return format!("crate{}", rest);
        }
        if let Some(rest) = module.strip_prefix("self::") {
            return join_module(&here, rest);
        }
        if module == "self" {
            return here;
        }
        if module.starts_with("super") {
            let mut base = here;
            let mut rest = module;
            while let Some(tail) = rest.strip_prefix("super") {
                base = parent_module(&base);
                rest = tail.strip_prefix("::").unwrap_or("");
                if rest.is_empty() {
                    return base;
                }
                if !rest.starts_with("super") {
                    return join_module(&base, rest);
                }
            }
            return base;
        }

        // A bare path is ambiguous. `use user::UserPayload;` inside
        // `models/mod.rs` means the sibling module `models::user` (always in
        // edition 2015, and still widely written), while `use serde::Serialize`
        // names an external crate. Prefer a sibling that actually exists, then
        // a top-level module, then treat it as external.
        let root = first_segment(module);

        let sibling = join_module(&here, root);
        if self.module_to_file.contains_key(&sibling) {
            return join_module(&here, module);
        }

        if self.crate_roots.contains_key(root) {
            return join_module("crate", module);
        }
        module.to_string()
    }

    /// True if the path names something inside this crate.
    fn looks_local(&self, absolute: &str) -> bool {
        absolute == "crate" || absolute.starts_with("crate::")
    }
}

/// Derive a module path from a file path, following Rust's layout rules.
fn file_to_module_path(file: &str) -> String {
    let path = file
        .trim_start_matches("./")
        .strip_suffix(".rs")
        .unwrap_or(file);

    // `src/` is conventional but not required (build scripts, examples).
    let path = path.strip_prefix("src/").unwrap_or(path);

    // `foo/mod.rs` is the module `foo`, not `foo::mod`.
    let path = path.strip_suffix("/mod").unwrap_or(path);

    if path == "lib" || path == "main" || path == "mod" || path.is_empty() {
        return "crate".to_string();
    }
    format!("crate::{}", path.replace('/', "::"))
}

fn join_module(base: &str, rest: &str) -> String {
    if rest.is_empty() {
        base.to_string()
    } else if base.is_empty() {
        rest.to_string()
    } else {
        format!("{base}::{rest}")
    }
}

fn parent_module(module: &str) -> String {
    match module.rfind("::") {
        Some(cut) => module[..cut].to_string(),
        None => "crate".to_string(),
    }
}

fn first_segment(path: &str) -> &str {
    path.split("::").next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_paths_follow_rust_layout() {
        assert_eq!(file_to_module_path("src/lib.rs"), "crate");
        assert_eq!(file_to_module_path("src/main.rs"), "crate");
        assert_eq!(file_to_module_path("src/models/mod.rs"), "crate::models");
        assert_eq!(
            file_to_module_path("src/models/user_payload.rs"),
            "crate::models::user_payload"
        );
    }

    #[test]
    fn super_paths_climb_from_the_current_module() {
        let tree = ModuleTree::default();

        // `src/mappers/deep/view.rs` is the module `crate::mappers::deep::view`,
        // so one `super` reaches `crate::mappers::deep` and two reach
        // `crate::mappers`. Note this differs from Python, where the first dot
        // of a relative import already means the containing package.
        assert_eq!(
            tree.absolutize("src/mappers/deep/view.rs", "super::super::models"),
            "crate::mappers::models"
        );
        assert_eq!(
            tree.absolutize("src/mappers/view.rs", "super::models::user"),
            "crate::mappers::models::user"
        );
        assert_eq!(
            tree.absolutize("src/mappers/deep/view.rs", "super::sibling"),
            "crate::mappers::deep::sibling"
        );

        assert_eq!(
            tree.absolutize("src/a/b.rs", "self::inner"),
            "crate::a::b::inner"
        );
        assert_eq!(tree.absolutize("src/a/b.rs", "crate::x"), "crate::x");
    }
}
