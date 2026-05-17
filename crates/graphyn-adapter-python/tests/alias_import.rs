use std::path::{Path, PathBuf};

use graphyn_adapter_python::analyze_files;
use graphyn_core::ir::RelationshipKind;

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/adapter-py/{name}"))
}

fn all_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in walkdir::WalkDir::new(root).into_iter().flatten() {
        if e.path().is_file()
            && matches!(
                e.path().extension().and_then(|x| x.to_str()),
                Some("py" | "pyi")
            )
        {
            out.push(e.path().to_path_buf());
        }
    }
    out
}

#[test]
fn test_python_alias_import_fixture_is_resolved_with_property_access() {
    let root = fixture_root("alias_import_bug");
    let repo_ir = analyze_files(&root, &all_files(&root)).expect("analysis must succeed");

    let mapper = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("view_model_mapper.py"))
        .expect("mapper file exists");

    let rel = mapper
        .relationships
        .iter()
        .find(|r| r.kind == RelationshipKind::Imports && r.alias.as_deref() == Some("ResponseModel"))
        .expect("aliased import exists");

    assert!(rel.to.ends_with("models/user_payload.py::UserPayload::class"));
    assert!(rel.properties_accessed.contains(&"user_id".to_string()));
    assert!(rel.properties_accessed.contains(&"timestamp".to_string()));
    assert!(rel.properties_accessed.contains(&"status".to_string()));
}
