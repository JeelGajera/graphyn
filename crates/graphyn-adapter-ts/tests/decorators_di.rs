use std::path::PathBuf;

use graphyn_adapter_ts::analyze_files;
use graphyn_adapter_ts::language::is_supported_source_file;
use graphyn_core::graph::GraphynGraph;
use graphyn_core::ir::RelationshipKind;
use graphyn_core::query::blast_radius;
use graphyn_core::scan::{walk_source_files_with_config, ScanConfig};

fn analyze_repo(
    root: &std::path::Path,
) -> Result<graphyn_core::ir::RepoIR, graphyn_adapter_ts::AdapterTsError> {
    let files = walk_source_files_with_config(
        root,
        &ScanConfig::default_enabled(),
        is_supported_source_file,
    )
    .expect("scan should succeed for NestJS fixture");
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
fn test_module_decorator_providers_emit_uses_type_relationships() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-ts/nestjs_di/src");
    let repo_ir = analyze_repo(&root).expect("analysis should succeed for NestJS DI fixture");

    let app_module = repo_ir
        .files
        .iter()
        .find(|f| f.file.ends_with("app.module.ts"))
        .expect("app.module.ts should be present in analyzed output");

    assert!(
        app_module.relationships.iter().any(|r| {
            r.kind == RelationshipKind::UsesType
                && r.to.ends_with("user.service.ts::UserService::class")
        }),
        "@Module providers should create UsesType relationship to UserService"
    );
    assert!(
        app_module.relationships.iter().any(|r| {
            r.kind == RelationshipKind::UsesType
                && r.to.ends_with("user.repository.ts::UserRepository::class")
        }),
        "@Module providers should create UsesType relationship to UserRepository"
    );
}

#[test]
fn test_blast_radius_user_repository_includes_app_module_via_module_decorator() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-ts/nestjs_di/src");
    let repo_ir = analyze_repo(&root).expect("analysis should succeed for NestJS DI fixture");
    let graph = build_graph(&repo_ir);

    let edges = blast_radius(&graph, "UserRepository", None, Some(2))
        .expect("blast radius query should resolve UserRepository");
    assert!(
        edges.iter().any(|edge| edge.from.contains("AppModule")),
        "AppModule should appear in blast radius via @Module providers reference"
    );
}
