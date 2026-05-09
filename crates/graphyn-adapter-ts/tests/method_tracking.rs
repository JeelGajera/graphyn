use std::path::PathBuf;

use graphyn_adapter_ts::analyze_files;
use graphyn_adapter_ts::language::is_supported_source_file;
use graphyn_core::graph::GraphynGraph;
use graphyn_core::ir::RelationshipKind;
use graphyn_core::query::dependencies;
use graphyn_core::scan::{walk_source_files_with_config, ScanConfig};

fn analyze_repo(
    root: &std::path::Path,
) -> Result<graphyn_core::ir::RepoIR, graphyn_adapter_ts::AdapterTsError> {
    let files = walk_source_files_with_config(
        root,
        &ScanConfig::default_enabled(),
        is_supported_source_file,
    )
    .expect("scan should succeed for method tracking fixture");
    analyze_files(root, &files)
}

fn build_graph(repo_ir: &graphyn_core::ir::RepoIR) -> GraphynGraph {
    let mut graph = GraphynGraph::new();
    for file in &repo_ir.files {
        for symbol in &file.symbols {
            graph.add_symbol(symbol.clone());
        }
    }
    for file in &repo_ir.files {
        for relationship in &file.relationships {
            graph.add_relationship(relationship);
        }
    }
    graph
}

#[test]
fn test_method_level_relationships_are_emitted_for_property_accesses() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/adapter-ts/di_injection/src");
    let repo_ir = analyze_repo(&root).expect("analysis should succeed for DI fixture");

    let payment = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("payment.service.ts"))
        .expect("payment service file should be present");

    assert!(
        payment.relationships.iter().any(|r| {
            r.kind == RelationshipKind::AccessesProperty
                && r.from.contains("::processPayment::method")
                && r.to.ends_with("user.repository.ts::UserRepository::class")
        }),
        "processPayment method should have a method-scoped relationship to UserRepository"
    );
    assert!(
        payment.relationships.iter().any(|r| {
            r.kind == RelationshipKind::AccessesProperty
                && r.from.contains("::processPayment::method")
                && r.to.ends_with("email.service.ts::EmailService::class")
        }),
        "processPayment method should have a method-scoped relationship to EmailService"
    );
}

#[test]
fn test_dependencies_for_process_payment_include_injected_services() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/adapter-ts/di_injection/src");
    let repo_ir = analyze_repo(&root).expect("analysis should succeed for DI fixture");
    let graph = build_graph(&repo_ir);

    let deps = dependencies(&graph, "processPayment", None, Some(2))
        .expect("dependencies query should resolve processPayment method");
    assert!(
        deps.iter()
            .any(|d| d.to.ends_with("user.repository.ts::UserRepository::class")),
        "processPayment dependencies should include UserRepository"
    );
    assert!(
        deps.iter()
            .any(|d| d.to.ends_with("email.service.ts::EmailService::class")),
        "processPayment dependencies should include EmailService"
    );
}
