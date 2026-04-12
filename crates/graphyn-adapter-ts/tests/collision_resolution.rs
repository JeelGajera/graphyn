use std::path::PathBuf;

use graphyn_adapter_ts::analyze_repo;
use graphyn_core::ir::RelationshipKind;

#[test]
fn test_unresolved_local_type_does_not_bind_to_random_global_symbol() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-ts/collision/src");
    let repo_ir = analyze_repo(&root).expect("repo analysis must succeed");

    let usage = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("usage.ts"))
        .expect("usage file exists");

    let prop_rel = usage
        .relationships
        .iter()
        .find(|r| r.kind == RelationshipKind::AccessesProperty)
        .expect("property access relationship exists");

    assert!(prop_rel.to.starts_with("__UNRESOLVED_LOCAL_TYPE__|Payload"));
    assert!(usage
        .parse_errors
        .iter()
        .any(|e| e.contains("unable to resolve property-access type 'Payload'")));
}
