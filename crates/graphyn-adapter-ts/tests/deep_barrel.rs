use std::path::PathBuf;

use graphyn_adapter_ts::analyze_files;
use graphyn_adapter_ts::language::is_supported_source_file;
use graphyn_core::ir::RelationshipKind;
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

#[test]
fn test_deep_barrel_3_hop_resolves() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-ts/deep_barrel/src");
    let repo_ir = analyze_repo(&root).expect("analysis succeeds");

    let app = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("app.ts"))
        .expect("app.ts exists");

    // renderBlockquote should resolve through:
    // app.ts -> lib/index.ts -> lib/renderers/index.ts -> lib/renderers/blockquote.ts
    let bq_import = app
        .relationships
        .iter()
        .find(|r| r.kind == RelationshipKind::Imports && r.to.contains("renderBlockquote"))
        .expect("renderBlockquote import should resolve");
    assert!(
        bq_import.to.contains("blockquote.ts"),
        "Should resolve to blockquote.ts through 3-hop chain, got: {}",
        bq_import.to
    );
}

#[test]
fn test_deep_barrel_no_unresolved_errors() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-ts/deep_barrel/src");
    let repo_ir = analyze_repo(&root).expect("analysis succeeds");

    let app = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("app.ts"))
        .expect("app.ts exists");

    let unresolved: Vec<_> = app
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("unable to resolve"))
        .collect();
    assert!(
        unresolved.is_empty(),
        "3-hop barrel should resolve without errors, got: {:?}",
        unresolved
    );
}
