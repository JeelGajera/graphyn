use std::collections::HashMap;
use std::path::{Path, PathBuf};

use graphyn_core::ir::{FileIR, Language, RepoIR};
use graphyn_core::scan::detect_language_from_extension;

#[derive(Debug)]
pub enum DispatchError {
    Ts(String),
    Python(String),
    Rust(String),
    Go(String),
    C(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ts(e) => write!(f, "TypeScript adapter error: {e}"),
            Self::Python(e) => write!(f, "Python adapter error: {e}"),
            Self::Rust(e) => write!(f, "Rust adapter error: {e}"),
            Self::Go(e) => write!(f, "Go adapter error: {e}"),
            Self::C(e) => write!(f, "C/C++ adapter error: {e}"),
        }
    }
}

impl std::error::Error for DispatchError {}

pub fn analyze_files(root: &Path, files: &[PathBuf]) -> Result<RepoIR, DispatchError> {
    let mut by_language: HashMap<Language, Vec<PathBuf>> = HashMap::new();
    for file in files {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            if let Some(lang) = detect_language_from_extension(ext) {
                by_language.entry(lang).or_default().push(file.clone());
            }
        }
    }

    let mut all_file_irs: Vec<FileIR> = Vec::new();
    let mut language_stats: HashMap<String, usize> = HashMap::new();

    for (language, lang_files) in by_language {
        let file_irs = match language {
            Language::TypeScript | Language::JavaScript => graphyn_adapter_ts::analyze_files(root, &lang_files)
                .map_err(|e| DispatchError::Ts(e.to_string()))?
                .files,
            Language::Python => graphyn_adapter_python::analyze_files(root, &lang_files)
                .map_err(|e| DispatchError::Python(e.to_string()))?
                .files,
            Language::Rust => graphyn_adapter_rust::analyze_files(root, &lang_files)
                .map_err(|e| DispatchError::Rust(e.to_string()))?
                .files,
            Language::Go => graphyn_adapter_go::analyze_files(root, &lang_files)
                .map_err(|e| DispatchError::Go(e.to_string()))?
                .files,
            Language::C | Language::Cpp => graphyn_adapter_c::analyze_files(root, &lang_files)
                .map_err(|e| DispatchError::C(e.to_string()))?
                .files,
            _ => continue,
        };

        *language_stats.entry(format!("{language:?}")).or_insert(0) += file_irs.len();
        all_file_irs.extend(file_irs);
    }

    Ok(RepoIR {
        root: root.to_string_lossy().to_string(),
        files: all_file_irs,
        language_stats,
    })
}
