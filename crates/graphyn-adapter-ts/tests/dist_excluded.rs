use std::path::PathBuf;

use graphyn_adapter_ts::analyze_files;
use graphyn_adapter_ts::language::is_supported_source_file;
use graphyn_core::scan::{walk_source_files_with_config, ScanConfig};

#[test]
fn test_dist_and_declaration_files_excluded() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-ts/dist_excluded");
    let files = walk_source_files_with_config(
        &root,
        &ScanConfig::default_enabled(),
        is_supported_source_file,
    )
    .unwrap();

    let relative_paths: Vec<String> = files
        .iter()
        .map(|f| {
            f.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();

    // src/app.ts should be included
    assert!(
        relative_paths.iter().any(|f| f.contains("src/app.ts")),
        "src/app.ts should be included, got: {:?}",
        relative_paths
    );

    // dist/ files should be excluded
    assert!(
        !relative_paths.iter().any(|f| f.contains("dist/")),
        "dist/ files should be excluded, got: {:?}",
        relative_paths
    );
}

#[test]
fn test_excluded_files_produce_no_symbols() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-ts/dist_excluded");
    let files = walk_source_files_with_config(
        &root,
        &ScanConfig::default_enabled(),
        is_supported_source_file,
    )
    .unwrap();

    let repo_ir = analyze_files(&root, &files).expect("analysis succeeds");

    // Only src/app.ts should be indexed
    assert_eq!(repo_ir.files.len(), 1, "only src/app.ts should be indexed");
    assert!(repo_ir.files[0].file.ends_with("app.ts"));
}
