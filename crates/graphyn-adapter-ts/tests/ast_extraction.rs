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

use graphyn_core::ir::RelationshipKind;

#[test]
fn test_multiline_import_and_property_access_are_extracted() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/adapter-ts/multiline_import/src");
    let repo_ir = analyze_repo(&root).expect("repo analysis must succeed");

    let use_file = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("use.ts"))
        .expect("use.ts exists");

    let import_rel = use_file
        .relationships
        .iter()
        .find(|r| r.kind == RelationshipKind::Imports && r.alias.as_deref() == Some("AuthSession"))
        .expect("multiline import relationship exists");
    assert!(import_rel.to.ends_with("model.ts::Session::interface"));

    let prop_rel = use_file
        .relationships
        .iter()
        .find(|r| r.kind == RelationshipKind::AccessesProperty)
        .expect("property relationship exists");
    assert!(prop_rel.to.ends_with("model.ts::Session::interface"));
    assert_eq!(
        prop_rel.properties_accessed,
        vec!["token".to_string(), "userId".to_string()]
    );
}
