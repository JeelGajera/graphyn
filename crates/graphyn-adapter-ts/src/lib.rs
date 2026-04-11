use std::collections::HashMap;
use std::path::Path;

use graphyn_core::ir::RepoIR;

pub mod extractor;
pub mod import_resolver;
pub mod language;
pub mod parser;
pub mod walker;

#[derive(Debug)]
pub enum AdapterTsError {
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for AdapterTsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Parse(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl std::error::Error for AdapterTsError {}

impl From<std::io::Error> for AdapterTsError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn analyze_repo(root: &Path) -> Result<RepoIR, AdapterTsError> {
    let files = walker::walk_source_files(root)?;
    let mut file_irs = Vec::new();
    let mut language_stats: HashMap<String, usize> = HashMap::new();

    for path in files {
        let parsed = parser::parse_file(root, &path).map_err(AdapterTsError::Parse)?;
        let mut file_ir = extractor::extract_file_ir(&parsed);
        if file_ir.parse_errors.is_empty() {
            file_ir.parse_errors.extend(parsed.parse_errors);
        }
        *language_stats
            .entry(format!("{:?}", file_ir.language))
            .or_insert(0) += 1;
        file_irs.push(file_ir);
    }

    let mut repo_ir = RepoIR {
        root: root.to_string_lossy().to_string(),
        files: file_irs,
        language_stats,
    };

    import_resolver::resolve_repo_ir(root, &mut repo_ir);

    Ok(repo_ir)
}
