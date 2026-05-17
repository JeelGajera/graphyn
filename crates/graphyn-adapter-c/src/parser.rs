use std::path::{Path, PathBuf};

use graphyn_core::ir::Language;

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub file: String,
    pub source: String,
    pub tree: tree_sitter::Tree,
    pub language: Language,
    pub is_cpp: bool,
}

pub fn parse_file(root: &Path, path: &PathBuf) -> Result<ParsedFile, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("failed reading {}: {e}", path.display()))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let is_cpp = matches!(ext, "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh");

    let mut parser = tree_sitter::Parser::new();
    let language = if is_cpp {
        tree_sitter_cpp::language()
    } else {
        tree_sitter_c::language()
    };
    parser
        .set_language(&language)
        .map_err(|e| format!("failed to set language: {e}"))?;
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| "tree-sitter returned no parse tree".to_string())?;

    Ok(ParsedFile {
        file: path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"),
        source,
        tree,
        language: if is_cpp { Language::Cpp } else { Language::C },
        is_cpp,
    })
}
