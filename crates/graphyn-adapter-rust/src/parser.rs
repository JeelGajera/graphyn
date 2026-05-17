use std::path::{Path, PathBuf};

use graphyn_core::ir::Language;

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub file: String,
    pub source: String,
    pub tree: tree_sitter::Tree,
    pub language: Language,
}

pub fn parse_file(root: &Path, path: &PathBuf) -> Result<ParsedFile, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("failed reading {}: {e}", path.display()))?;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::language())
        .map_err(|e| format!("failed to set Rust language: {e}"))?;
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
        language: Language::Rust,
    })
}
