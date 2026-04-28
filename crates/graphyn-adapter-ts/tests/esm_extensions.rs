use std::path::PathBuf;

use graphyn_adapter_ts::analyze_files;
use graphyn_adapter_ts::language::is_supported_source_file;
use graphyn_core::ir::RelationshipKind;
use graphyn_core::scan::{walk_source_files_with_config, ScanConfig};

fn analyze(root: &std::path::Path) -> graphyn_core::ir::RepoIR {
    let files = walk_source_files_with_config(
        root,
        &ScanConfig::default_enabled(),
        is_supported_source_file,
    )
    .expect("scan succeeds");
    analyze_files(root, &files).expect("analysis succeeds")
}

#[test]
fn test_mts_cts_mjs_files_are_indexed_and_imports_resolve() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/adapter-ts/esm_extensions/src");
    let repo_ir = analyze(&root);

    assert!(
        repo_ir.files.iter().any(|f| f.file.ends_with("server.mts")),
        "server.mts should be indexed"
    );
    assert!(
        repo_ir
            .files
            .iter()
            .any(|f| f.file.ends_with("handler.cts")),
        "handler.cts should be indexed"
    );
    assert!(
        repo_ir.files.iter().any(|f| f.file.ends_with("config.mjs")),
        "config.mjs should be indexed"
    );

    let server_file = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("server.mts"))
        .expect("server.mts file should exist");

    let import_edge = server_file.relationships.iter().find(|r| {
        r.kind == RelationshipKind::Imports
            && r.to.contains("handler")
            && r.to.contains("handler.cts")
    });
    assert!(
        import_edge.is_some(),
        "import from server.mts should resolve to handler.cts, got: {:?}",
        server_file
            .relationships
            .iter()
            .map(|r| r.to.as_str())
            .collect::<Vec<_>>()
    );
}
