use std::path::{Path, PathBuf};

use graphyn_core::ir::Language;

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub file: String,
    pub source: String,
    pub tree: tree_sitter::Tree,
    pub language: Language,
    pub package_name: String,
}

pub fn parse_file(root: &Path, path: &PathBuf) -> Result<ParsedFile, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("failed reading {}: {e}", path.display()))?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_go::language())
        .map_err(|e| format!("failed to set Go language: {e}"))?;
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| "tree-sitter returned no parse tree".to_string())?;

    let mut package_name = String::new();
    {
        let root_node = tree.root_node();
        let mut cursor = root_node.walk();
        for child in root_node.children(&mut cursor) {
            if child.kind() == "package_clause" {
                let mut pc = child.walk();
                for n in child.children(&mut pc) {
                    if n.kind() == "package_identifier" || n.kind() == "identifier" {
                        package_name = n.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                }
            }
        }
    }

    Ok(ParsedFile {
        file: path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"),
        source,
        tree,
        language: Language::Go,
        package_name,
    })
}
