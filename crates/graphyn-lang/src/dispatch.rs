//! Routing files to the language module that understands them.
//!
//! Each adapter resolves imports within its own language, so files are grouped
//! by language and each group analysed as a unit. Grouping is ordered rather
//! than hash-ordered: `RepoIR.files` determines the order symbols and edges are
//! inserted into the graph, and Graphyn's first documented guarantee is that
//! the graph is deterministic. `HashMap` iteration order varies per process,
//! which made two runs over identical input produce differently-ordered output.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graphyn_core::ir::{FileIR, Language, RepoIR};
use graphyn_core::scan::detect_language_from_extension;
use rayon::prelude::*;

#[derive(Debug)]
pub enum DispatchError {
    #[cfg(feature = "typescript")]
    Ts(String),
    #[cfg(feature = "python")]
    Python(String),
    #[cfg(feature = "rust")]
    Rust(String),
    #[cfg(feature = "go")]
    Go(String),
    #[cfg(feature = "c")]
    C(String),
    /// A Tier 2 language, analysed structurally.
    Structural(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "typescript")]
            Self::Ts(e) => write!(f, "TypeScript adapter error: {e}"),
            #[cfg(feature = "python")]
            Self::Python(e) => write!(f, "Python adapter error: {e}"),
            #[cfg(feature = "rust")]
            Self::Rust(e) => write!(f, "Rust adapter error: {e}"),
            #[cfg(feature = "go")]
            Self::Go(e) => write!(f, "Go adapter error: {e}"),
            #[cfg(feature = "c")]
            Self::C(e) => write!(f, "C/C++ adapter error: {e}"),
            Self::Structural(e) => write!(f, "structural analysis error: {e}"),
        }
    }
}

impl std::error::Error for DispatchError {}

/// A stable ordering key for a language group.
///
/// `Language` is not `Ord`, and deriving it would change a public enum's API
/// for an internal need, so the ordering lives here.
fn language_rank(language: &Language) -> u8 {
    match language {
        Language::TypeScript => 0,
        Language::JavaScript => 1,
        Language::Python => 2,
        Language::Rust => 3,
        Language::Go => 4,
        Language::C => 5,
        Language::Cpp => 6,
        Language::Java => 7,
    }
}

/// The adapter that owns a language, or `None` if it is not supported yet.
///
/// TypeScript and JavaScript share an adapter, and C and C++ share one, so the
/// grouping key is the adapter rather than the language.
fn adapter_group(language: &Language) -> Option<Language> {
    match language {
        #[cfg(feature = "typescript")]
        Language::TypeScript | Language::JavaScript => Some(Language::TypeScript),
        #[cfg(feature = "python")]
        Language::Python => Some(Language::Python),
        #[cfg(feature = "rust")]
        Language::Rust => Some(Language::Rust),
        #[cfg(feature = "go")]
        Language::Go => Some(Language::Go),
        #[cfg(feature = "c")]
        Language::C | Language::Cpp => Some(Language::C),
        #[cfg(feature = "java")]
        Language::Java => Some(Language::Java),
        // A language whose feature is off is skipped, so a slim build ignores
        // files it cannot analyse rather than failing on them.
        _ => None,
    }
}

pub fn analyze_files(root: &Path, files: &[PathBuf]) -> Result<RepoIR, DispatchError> {
    let mut by_adapter: BTreeMap<u8, (Language, Vec<PathBuf>)> = BTreeMap::new();

    for file in files {
        let Some(language) = file
            .extension()
            .and_then(|e| e.to_str())
            .and_then(detect_language_from_extension)
        else {
            continue;
        };
        let Some(group) = adapter_group(&language) else {
            continue;
        };
        by_adapter
            .entry(language_rank(&group))
            .or_insert_with(|| (group, Vec::new()))
            .1
            .push(file.clone());
    }

    // Each adapter resolves its own language independently, so groups can run
    // in parallel. Results are collected back in the deterministic key order.
    let groups: Vec<(Language, Vec<PathBuf>)> = by_adapter.into_values().collect();

    let analyzed: Vec<Result<Vec<FileIR>, DispatchError>> = groups
        .into_par_iter()
        .map(|(language, mut group_files)| {
            // Adapters index by path; a stable input order keeps their own
            // first-wins tie-breaks reproducible.
            group_files.sort();
            run_adapter(&language, root, &group_files)
        })
        .collect();

    let mut all_files: Vec<FileIR> = Vec::with_capacity(files.len());
    let mut language_stats: BTreeMap<String, usize> = BTreeMap::new();

    for result in analyzed {
        for file_ir in result? {
            *language_stats
                .entry(format!("{:?}", file_ir.language))
                .or_insert(0) += 1;
            all_files.push(file_ir);
        }
    }

    Ok(RepoIR {
        root: root.to_string_lossy().to_string(),
        files: all_files,
        language_stats,
    })
}

fn run_adapter(
    language: &Language,
    root: &Path,
    files: &[PathBuf],
) -> Result<Vec<FileIR>, DispatchError> {
    // A Tier 2 language needs no arm here: one generic implementation covers
    // every one of them, driven by the grammar's own tags query. That is the
    // whole point of the tier — adding a structural language is a dependency,
    // a feature flag and a spec, not a pipeline.
    if let Some(spec) = crate::spec::for_language(language) {
        if spec.tier() == crate::spec::Tier::Structural {
            return crate::structural::analyze(spec, root, files)
                .map_err(DispatchError::Structural);
        }
    }

    // Each remaining arm is gated by its language's feature. `adapter_group`
    // already refuses to route to a language this build does not carry, so an
    // arm that is compiled out is unreachable rather than silently skipped.
    Ok(match language {
        #[cfg(feature = "typescript")]
        Language::TypeScript => {
            crate::lang::typescript::analyze_files(root, files)
                .map_err(|e| DispatchError::Ts(e.to_string()))?
                .files
        }
        #[cfg(feature = "python")]
        Language::Python => {
            crate::lang::python::analyze_files(root, files)
                .map_err(|e| DispatchError::Python(e.to_string()))?
                .files
        }
        #[cfg(feature = "rust")]
        Language::Rust => {
            crate::lang::rust::analyze_files(root, files)
                .map_err(|e| DispatchError::Rust(e.to_string()))?
                .files
        }
        #[cfg(feature = "go")]
        Language::Go => {
            crate::lang::go::analyze_files(root, files)
                .map_err(|e| DispatchError::Go(e.to_string()))?
                .files
        }
        #[cfg(feature = "c")]
        Language::C => {
            crate::lang::c::analyze_files(root, files)
                .map_err(|e| DispatchError::C(e.to_string()))?
                .files
        }
        // `adapter_group` never yields anything else.
        _ => Vec::new(),
    })
}

/// Languages this build can analyse, for help text and diagnostics.
///
/// Built from the enabled features rather than hardcoded, so a slim build
/// reports what it can actually do. The list was always accurate before only
/// because there was exactly one possible build.
pub fn supported_languages() -> Vec<crate::spec::LanguageSupport> {
    crate::spec::specs()
        .into_iter()
        .map(|s| crate::spec::LanguageSupport {
            name: s.name(),
            tier: s.tier(),
        })
        .collect()
}

/// Just the names, for callers that only need a list.
pub fn supported_language_names() -> Vec<&'static str> {
    crate::spec::specs().into_iter().map(|s| s.name()).collect()
}
