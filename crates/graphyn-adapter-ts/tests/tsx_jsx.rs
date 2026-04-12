use std::path::PathBuf;

use graphyn_adapter_ts::analyze_repo;

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
