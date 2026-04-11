use std::path::PathBuf;

use graphyn_adapter_ts::analyze_repo;
use graphyn_core::ir::RelationshipKind;

#[test]
fn test_reexports_and_barrel_are_resolved() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-ts/re_exports/src");
    let repo_ir = analyze_repo(&root).expect("repo analysis must succeed");

    let index_file = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("index.ts"))
        .expect("index file exists");

    let named_reexport = index_file
        .relationships
        .iter()
        .find(|r| r.kind == RelationshipKind::ReExports && r.alias.as_deref() == Some("PublicUser"))
        .expect("named re-export with alias exists");
    assert!(named_reexport
        .to
        .ends_with("user_payload.ts::UserPayload::class"));

    let barrel_reexports: Vec<_> = index_file
        .relationships
        .iter()
        .filter(|r| r.kind == RelationshipKind::ReExports && r.context.contains("export *"))
        .collect();
    assert!(!barrel_reexports.is_empty());
}
