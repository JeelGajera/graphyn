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

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/alias-import-bug/src")
}

#[test]
fn test_alias_import_fixture_is_resolved_with_property_access() {
    let repo_ir = analyze_repo(&fixture_root()).expect("repo analysis must succeed");

    let mapper_file = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("mappers/deep/view_model_mapper.ts"))
        .expect("mapper file exists");

    let rel = mapper_file
        .relationships
        .iter()
        .find(|r| {
            r.kind == RelationshipKind::Imports && r.alias.as_deref() == Some("ResponseModel")
        })
        .expect("aliased import relationship exists");

    assert!(rel
        .to
        .ends_with("models/user_payload.ts::UserPayload::class"));
    assert_eq!(
        rel.properties_accessed,
        vec![
            "status".to_string(),
            "timestamp".to_string(),
            "userId".to_string()
        ]
    );

    let prop_rel = mapper_file
        .relationships
        .iter()
        .find(|r| r.kind == RelationshipKind::AccessesProperty)
        .expect("property access relationship exists");
    assert!(prop_rel
        .to
        .ends_with("models/user_payload.ts::UserPayload::class"));
}
