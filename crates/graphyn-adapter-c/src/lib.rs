use std::collections::HashMap;
use std::path::{Path, PathBuf};

use graphyn_core::ir::RepoIR;
use rayon::prelude::*;

pub mod extractor;
pub mod include_resolver;
pub mod namespace_resolver;
pub mod parser;
pub mod preprocessor;
pub mod scope_analyzer;
pub mod template_analyzer;

#[derive(Debug)]
pub enum AdapterCError {
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for AdapterCError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Parse(err) => write!(f, "parse error: {err}"),
        }
    }
}
impl std::error::Error for AdapterCError {}

pub fn analyze_files(root: &Path, files: &[PathBuf]) -> Result<RepoIR, AdapterCError> {
    let parse_results: Vec<Result<_, AdapterCError>> = files
        .par_iter()
        .map(|path| {
            let parsed = parser::parse_file(root, path).map_err(AdapterCError::Parse)?;
            Ok(extractor::extract_file_ir(&parsed))
        })
        .collect();

    let mut file_irs = Vec::new();
    let mut language_stats: HashMap<String, usize> = HashMap::new();
    for result in parse_results {
        let file_ir = result?;
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
    include_resolver::resolve_repo_ir(root, &mut repo_ir);
    Ok(repo_ir)
}
