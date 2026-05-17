use std::collections::HashMap;

use graphyn_core::ir::{FileIR, SymbolKind};

#[derive(Debug, Default)]
pub struct ModuleTree {
    pub path_to_symbol_id: HashMap<String, String>,
}

impl ModuleTree {
    pub fn build(files: &[FileIR]) -> Self {
        let mut t = Self::default();
        for f in files {
            let module_path = file_to_crate_path(&f.file);
            for s in &f.symbols {
                if s.kind == SymbolKind::Module {
                    continue;
                }
                let full = format!("{}::{}", module_path, s.name);
                t.path_to_symbol_id.insert(full, s.id.clone());
                t.path_to_symbol_id
                    .entry(s.name.clone())
                    .or_insert_with(|| s.id.clone());
            }
        }
        t
    }

    pub fn could_be_local(&self, path: &str) -> bool {
        let root = path.split("::").next().unwrap_or("");
        self.path_to_symbol_id.keys().any(|k| k.starts_with(root))
    }

    pub fn resolve_use_path(&self, _current_file: &str, use_path: &str) -> Option<String> {
        if let Some(id) = self.path_to_symbol_id.get(use_path) {
            return Some(id.clone());
        }
        let stripped = use_path
            .trim_start_matches("crate::")
            .trim_start_matches("super::")
            .trim_start_matches("self::");
        if let Some(id) = self.path_to_symbol_id.get(stripped) {
            return Some(id.clone());
        }
        let leaf = use_path.rsplit("::").next().unwrap_or(use_path);
        self.path_to_symbol_id.get(leaf).cloned()
    }
}

fn file_to_crate_path(file: &str) -> String {
    let without_src = file.trim_start_matches("src/");
    let without_ext = without_src.trim_end_matches(".rs");
    let without_mod = without_ext.trim_end_matches("/mod");
    if without_mod == "lib" || without_mod == "main" {
        "crate".to_string()
    } else {
        format!("crate::{}", without_mod.replace('/', "::"))
    }
}
