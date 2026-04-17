use std::path::PathBuf;

use graphyn_adapter_ts::analyze_files;
use graphyn_adapter_ts::language::is_supported_source_file;
use graphyn_core::scan::{walk_source_files_with_config, ScanConfig};

fn analyze_repo(
    root: &std::path::Path,
) -> Result<graphyn_core::ir::RepoIR, graphyn_adapter_ts::AdapterTsError> {
    let files = walk_source_files_with_config(
        root,
        &ScanConfig::default_enabled(),
        is_supported_source_file,
    )
    .unwrap();
    analyze_files(root, &files)
}

fn analyze_repo_with_config(
    root: &std::path::Path,
    config: &ScanConfig,
) -> Result<graphyn_core::ir::RepoIR, graphyn_adapter_ts::AdapterTsError> {
    let files = walk_source_files_with_config(root, config, is_supported_source_file).unwrap();
    analyze_files(root, &files)
}

#[test]
fn test_gitignore_is_respected_by_default() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/adapter-ts/ignore_filters/src");

    let repo_ir =
        analyze_repo_with_config(&root, &ScanConfig::default_enabled()).expect("analysis succeeds");

    let files: Vec<String> = repo_ir.files.iter().map(|f| f.file.clone()).collect();
    assert!(files.iter().any(|f| f.ends_with("included.ts")));
    assert!(!files.iter().any(|f| f.ends_with("ignored.ts")));
    assert!(!files.iter().any(|f| f.contains("ignored_dir/")));
}

#[test]
fn test_include_and_exclude_patterns_are_applied() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/adapter-ts/ignore_filters/src");

    let cfg = ScanConfig {
        include_patterns: vec!["included_dir/**".to_string()],
        exclude_patterns: vec!["**/keep.ts".to_string()],
        respect_gitignore: false,
    };

    let repo_ir = analyze_repo_with_config(&root, &cfg).expect("analysis succeeds");
    let files: Vec<String> = repo_ir.files.iter().map(|f| f.file.clone()).collect();

    assert!(!files.iter().any(|f| f.ends_with("included.ts")));
    assert!(!files.iter().any(|f| f.ends_with("included_dir/keep.ts")));
}
