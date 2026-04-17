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
fn test_tsx_and_jsx_files_parse_without_errors() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-ts/tsx_jsx/src");
    let repo_ir = analyze_repo(&root).expect("repo analysis must succeed");

    let tsx = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("component.tsx"))
        .expect("tsx file exists");
    let jsx = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("view.jsx"))
        .expect("jsx file exists");

    assert!(
        !tsx.parse_errors.iter().any(|e| e.starts_with("line ")),
        "tsx syntax parse errors: {:?}",
        tsx.parse_errors
    );
    assert!(
        !jsx.parse_errors.iter().any(|e| e.starts_with("line ")),
        "jsx syntax parse errors: {:?}",
        jsx.parse_errors
    );
    assert!(tsx.symbols.iter().any(|s| s.name == "Header"));
    assert!(jsx.symbols.iter().any(|s| s.name == "View"));
}
