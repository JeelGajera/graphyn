use std::path::Path;

use graphyn_core::ir::Language;
use tree_sitter::Tree;

use crate::language::detect_language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceDialect {
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
}

pub struct ParsedFile {
    pub file: String,
    pub language: Language,
    pub dialect: SourceDialect,
    pub source: String,
    pub tree: Tree,
    pub parse_errors: Vec<String>,
}

pub fn parse_file(root: &Path, path: &Path) -> Result<ParsedFile, String> {
    let dialect = detect_dialect(path)
        .ok_or_else(|| format!("unsupported file extension: {}", path.display()))?;
    let language = detect_language(path)
        .ok_or_else(|| format!("unsupported file extension: {}", path.display()))?;

    let source = std::fs::read_to_string(path)
        .map_err(|err| format!("failed reading {}: {err}", path.display()))?;
    let tree = parse_source(dialect, &source)?;

    let file = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    let parse_errors = collect_parse_errors(tree.root_node(), &source);

    Ok(ParsedFile {
        file,
        language,
        dialect,
        source,
        tree,
        parse_errors,
    })
}

fn parse_source(dialect: SourceDialect, source: &str) -> Result<Tree, String> {
    let mut parser = tree_sitter::Parser::new();
    let ts_language = match dialect {
        SourceDialect::TypeScript => tree_sitter_typescript::language_typescript(),
        SourceDialect::Tsx => tree_sitter_typescript::language_tsx(),
        // tree-sitter-javascript handles both JS and JSX for this crate version.
        SourceDialect::JavaScript | SourceDialect::Jsx => tree_sitter_javascript::language(),
    };

    parser
        .set_language(&ts_language)
        .map_err(|err| format!("failed to set parser language: {err}"))?;
    parser
        .parse(source, None)
        .ok_or_else(|| "tree-sitter returned no parse tree".to_string())
}

fn detect_dialect(path: &Path) -> Option<SourceDialect> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "ts" => Some(SourceDialect::TypeScript),
        "tsx" => Some(SourceDialect::Tsx),
        "js" => Some(SourceDialect::JavaScript),
        "jsx" => Some(SourceDialect::Jsx),
        _ => None,
    }
}

fn collect_parse_errors(node: tree_sitter::Node<'_>, source: &str) -> Vec<String> {
    let mut out = Vec::new();
    collect_parse_errors_recursive(node, source, &mut out);
    out.sort();
    out.dedup();
    out
}

fn collect_parse_errors_recursive(
    node: tree_sitter::Node<'_>,
    source: &str,
    out: &mut Vec<String>,
) {
    if node.is_error() {
        let row = node.start_position().row + 1;
        let line = source
            .lines()
            .nth(row.saturating_sub(1))
            .unwrap_or_default();
        out.push(format!("line {row}: {}", line.trim()));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_parse_errors_recursive(child, source, out);
    }
}
