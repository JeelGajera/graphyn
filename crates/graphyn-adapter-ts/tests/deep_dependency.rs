use std::path::PathBuf;

use graphyn_adapter_ts::analyze_repo;
use graphyn_core::ir::RelationshipKind;

#[test]
fn test_deep_relative_import_dependency_resolves() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/adapter-ts/deep_dependency/src");
    let repo_ir = analyze_repo(&root).expect("repo analysis must succeed");

    let mapper = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("a/b/mapper.ts"))
        .expect("mapper file exists");

    let rel = mapper
        .relationships
        .iter()
        .find(|r| r.kind == RelationshipKind::Imports && r.alias.as_deref() == Some("DeepAccount"))
        .expect("deep alias import exists");

    assert!(rel.to.ends_with("models/account.ts::Account::class"));
}
