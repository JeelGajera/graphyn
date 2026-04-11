use graphyn_core::graph::GraphynGraph;
use graphyn_core::ir::{Language, Relationship, RelationshipKind, Symbol, SymbolKind};
use graphyn_core::query::{blast_radius, symbol_usages};
use graphyn_core::resolver::{AliasResolver, AliasScope};

fn symbol(id: &str, name: &str, file: &str, kind: SymbolKind) -> Symbol {
    Symbol {
        id: id.to_string(),
        name: name.to_string(),
        kind,
        language: Language::TypeScript,
        file: file.to_string(),
        line_start: 1,
        line_end: 1,
        signature: None,
    }
}

#[test]
fn test_aliased_import_is_tracked_with_property_accesses() {
    let mut graph = GraphynGraph::new();

    let user_payload = symbol(
        "src/models/user_payload.ts::UserPayload::class",
        "UserPayload",
        "src/models/user_payload.ts",
        SymbolKind::Class,
    );
    let mapper = symbol(
        "src/mappers/response/deep/view_model_mapper.ts::ViewModelMapper::class",
        "ViewModelMapper",
        "src/mappers/response/deep/view_model_mapper.ts",
        SymbolKind::Class,
    );

    graph.add_symbol(user_payload.clone());
    graph.add_symbol(mapper.clone());

    let relationships = vec![Relationship {
        from: mapper.id.clone(),
        to: user_payload.id.clone(),
        kind: RelationshipKind::Imports,
        alias: Some("ResponseModel".to_string()),
        properties_accessed: vec![
            "userId".to_string(),
            "timestamp".to_string(),
            "status".to_string(),
        ],
        context: "import { UserPayload as ResponseModel } from '../../../models/user_payload';"
            .to_string(),
        file: "src/mappers/response/deep/view_model_mapper.ts".to_string(),
        line: 1,
    }];

    for relationship in &relationships {
        graph.add_relationship(relationship);
    }

    let resolver = AliasResolver::default();
    resolver.ingest_relationships(&graph, &relationships);

    let aliases = graph
        .alias_chains
        .get(&user_payload.id)
        .expect("alias chain exists");
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases[0].alias_name, "ResponseModel");
    assert_eq!(aliases[0].scope, AliasScope::ImportAlias);

    let blast = blast_radius(&graph, "UserPayload", None, Some(2)).expect("blast radius succeeds");
    assert_eq!(blast.len(), 1);
    assert_eq!(
        blast[0].file,
        "src/mappers/response/deep/view_model_mapper.ts"
    );
    assert_eq!(blast[0].alias.as_deref(), Some("ResponseModel"));
    assert_eq!(
        blast[0].properties_accessed,
        vec!["userId", "timestamp", "status"]
    );

    let usages = symbol_usages(&graph, "UserPayload", None, true).expect("usages succeeds");
    assert_eq!(usages.len(), 1);
    assert_eq!(usages[0].alias.as_deref(), Some("ResponseModel"));
}

#[test]
fn test_alias_resolver_supports_reexport_barrel_and_default_scopes() {
    let mut graph = GraphynGraph::new();

    let canonical = symbol(
        "src/models/user_payload.ts::UserPayload::class",
        "UserPayload",
        "src/models/user_payload.ts",
        SymbolKind::Class,
    );
    let barrel = symbol(
        "src/models/index.ts::Models::module",
        "Models",
        "src/models/index.ts",
        SymbolKind::Module,
    );

    graph.add_symbol(canonical.clone());
    graph.add_symbol(barrel.clone());

    let relationships = vec![
        Relationship {
            from: barrel.id.clone(),
            to: canonical.id.clone(),
            kind: RelationshipKind::ReExports,
            alias: Some("PublicUser".to_string()),
            properties_accessed: vec![],
            context: "export { UserPayload as PublicUser } from './user_payload'".to_string(),
            file: "src/models/index.ts".to_string(),
            line: 1,
        },
        Relationship {
            from: barrel.id.clone(),
            to: canonical.id.clone(),
            kind: RelationshipKind::ReExports,
            alias: Some("UserPayload".to_string()),
            properties_accessed: vec![],
            context: "export * from './user_payload'".to_string(),
            file: "src/models/index.ts".to_string(),
            line: 2,
        },
        Relationship {
            from: barrel.id.clone(),
            to: canonical.id.clone(),
            kind: RelationshipKind::Imports,
            alias: Some("User".to_string()),
            properties_accessed: vec![],
            context: "import User from './user_payload' // default".to_string(),
            file: "src/models/index.ts".to_string(),
            line: 3,
        },
    ];

    let resolver = AliasResolver::default();
    resolver.ingest_relationships(&graph, &relationships);

    let aliases = graph
        .alias_chains
        .get(&canonical.id)
        .expect("alias chain exists");
    assert_eq!(aliases.len(), 3);
    assert!(aliases
        .iter()
        .any(|a| a.alias_name == "PublicUser" && a.scope == AliasScope::ReExport));
    assert!(aliases
        .iter()
        .any(|a| a.alias_name == "UserPayload" && a.scope == AliasScope::BarrelReExport));
    assert!(aliases
        .iter()
        .any(|a| a.alias_name == "User" && a.scope == AliasScope::DefaultImport));

    assert_eq!(
        resolver
            .resolve_alias_in_file("PublicUser", "src/models/index.ts")
            .as_deref(),
        Some(canonical.id.as_str())
    );
}

#[test]
fn test_accesses_property_edge_can_be_canonicalized_from_alias() {
    let mut graph = GraphynGraph::new();
    let canonical = symbol(
        "src/models/user_payload.ts::UserPayload::class",
        "UserPayload",
        "src/models/user_payload.ts",
        SymbolKind::Class,
    );
    let mapper = symbol(
        "src/mappers/response/deep/view_model_mapper.ts::ViewModelMapper::class",
        "ViewModelMapper",
        "src/mappers/response/deep/view_model_mapper.ts",
        SymbolKind::Class,
    );
    graph.add_symbol(canonical.clone());
    graph.add_symbol(mapper.clone());

    let import_rel = Relationship {
        from: mapper.id.clone(),
        to: canonical.id.clone(),
        kind: RelationshipKind::Imports,
        alias: Some("ResponseModel".to_string()),
        properties_accessed: vec![],
        context: "import { UserPayload as ResponseModel } from '../../../models/user_payload';"
            .to_string(),
        file: "src/mappers/response/deep/view_model_mapper.ts".to_string(),
        line: 1,
    };
    let resolver = AliasResolver::default();
    resolver.ingest_relationships(&graph, &[import_rel]);

    let accesses_property = Relationship {
        from: mapper.id.clone(),
        to: "src/mappers/response/deep/view_model_mapper.ts::ResponseModel::alias".to_string(),
        kind: RelationshipKind::AccessesProperty,
        alias: Some("ResponseModel".to_string()),
        properties_accessed: vec!["userId".to_string()],
        context: "id: data.userId".to_string(),
        file: "src/mappers/response/deep/view_model_mapper.ts".to_string(),
        line: 6,
    };

    let normalized = resolver.canonicalize_relationship(&accesses_property);
    assert_eq!(normalized.to, canonical.id);
    assert_eq!(normalized.properties_accessed, vec!["userId"]);
}
