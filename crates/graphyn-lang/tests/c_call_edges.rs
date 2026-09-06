// Exercises the c module, so it compiles only when that language is
// enabled. A slim build otherwise tries to compile a test for a module it
// does not carry.
#![cfg(feature = "c")]

use std::path::{Path, PathBuf};

use graphyn_core::ir::{RelationshipKind, RepoIR};
use graphyn_lang::lang::c::analyze_files;

fn all_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().flatten() {
        if entry.path().is_file()
            && matches!(
                entry.path().extension().and_then(|x| x.to_str()),
                Some("c" | "h" | "cpp" | "hpp")
            )
        {
            out.push(entry.path().to_path_buf());
        }
    }
    out
}

fn analyze(root: &Path) -> RepoIR {
    analyze_files(root, &all_files(root)).expect("analysis must succeed")
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adapter-c/call_edges")
}

/// Every call-like edge in `suffix`, as (kind, target, line).
fn call_edges(ir: &RepoIR, suffix: &str) -> Vec<(RelationshipKind, String, u32)> {
    let mut out: Vec<(RelationshipKind, String, u32)> = ir
        .files
        .iter()
        .filter(|f| f.file.ends_with(suffix))
        .flat_map(|f| f.relationships.iter())
        .filter(|r| r.kind == RelationshipKind::Calls || r.kind == RelationshipKind::Instantiates)
        .map(|r| (r.kind.clone(), r.to.clone(), r.line))
        .collect();
    out.sort_by_key(|(_, to, line)| (*line, to.clone()));
    out
}

#[test]
fn a_call_to_a_function_defined_in_the_same_file_resolves() {
    let ir = analyze(&fixture());
    let edges = call_edges(&ir, "render.c");

    assert!(
        edges.iter().any(|(kind, to, _)| *kind == RelationshipKind::Calls
            && to.ends_with("render.c::scale::function")),
        "the same-file call to scale is missing: {edges:?}"
    );
}

#[test]
fn a_call_to_a_function_defined_in_an_included_header_resolves() {
    // Header-defined functions are the norm in C++ and common in C via
    // `static inline`, so this is the shape that carries most of the value
    // here.
    let ir = analyze(&fixture());
    let edges = call_edges(&ir, "draw.cpp");

    assert!(
        edges.iter().any(|(kind, to, _)| *kind == RelationshipKind::Calls
            && to.ends_with("shapes.hpp::area::function")),
        "the call into shapes.hpp is missing: {edges:?}"
    );
}

#[test]
fn new_records_an_instantiation() {
    // C has no construction syntax at all — `struct Foo f = {..}` is a
    // declaration with an initializer, not an expression naming a
    // constructor. `new Foo(..)` in C++ is the only shape that is
    // construction outright.
    let ir = analyze(&fixture());
    let edges = call_edges(&ir, "draw.cpp");

    assert!(
        edges.iter().any(|(kind, to, line)| *kind
            == RelationshipKind::Instantiates
            && to.ends_with("shapes.hpp::Circle::class")
            && *line == 5),
        "no instantiation for `new Circle()`: {edges:?}"
    );
}

#[test]
fn a_functional_cast_is_not_recorded_as_a_call() {
    // `Circle(*circle)` is spelled exactly like a call and calls nothing.
    // The resolved target's kind is the only thing that can tell them apart,
    // the same rule Python and Go need.
    let ir = analyze(&fixture());
    let edges = call_edges(&ir, "draw.cpp");

    let cast = edges
        .iter()
        .find(|(_, to, line)| to.ends_with("Circle::class") && *line == 10)
        .unwrap_or_else(|| panic!("no edge for the functional cast, got {edges:?}"));

    assert_eq!(
        cast.0,
        RelationshipKind::Instantiates,
        "a functional cast must not be a call, got {cast:?}"
    );
}

#[test]
fn a_call_through_a_header_prototype_records_nothing() {
    // The documented gap, pinned so it cannot change silently in either
    // direction. `geometry.h` *declares* `point_distance` without defining
    // it, so no symbol in the graph carries that name and there is nothing
    // to point an edge at. Linking a prototype to the definition in another
    // translation unit is a separate change with graph-shape consequences.
    //
    // This is the dominant call shape in plain C, so the test exists to keep
    // the limitation visible rather than to bless it.
    let ir = analyze(&fixture());
    let edges = call_edges(&ir, "render.c");

    assert!(
        edges.iter().all(|(_, to, _)| !to.contains("point_distance")),
        "a header prototype resolved; the documented limit is now wrong: {edges:?}"
    );
}

#[test]
fn a_standard_library_call_records_no_edge_and_no_diagnostic() {
    // `printf` names nothing in this graph. No edge, and no diagnostic:
    // there is nothing here a user could fix, and a warning per `printf`
    // would bury the resolution warnings that matter.
    let ir = analyze(&fixture());
    let edges = call_edges(&ir, "render.c");

    assert!(
        edges.iter().all(|(_, to, _)| !to.contains("printf")),
        "a standard-library call produced an edge: {edges:?}"
    );

    let render = ir
        .files
        .iter()
        .find(|f| f.file.ends_with("render.c"))
        .expect("render.c in the analysis");
    assert!(
        render.diagnostics.is_empty(),
        "unresolvable calls must not raise diagnostics, got {:?}",
        render.diagnostics
    );
}

#[test]
fn a_member_call_is_not_recorded_as_a_call_to_the_receiver() {
    // `circle->radius` is a property access on Circle; a call edge to the
    // type would claim the type was called.
    let ir = analyze(&fixture());

    for suffix in ["render.c", "draw.cpp"] {
        let edges = call_edges(&ir, suffix);
        assert!(
            edges
                .iter()
                .all(|(kind, to, _)| *kind != RelationshipKind::Calls || !to.ends_with("::class")),
            "{suffix}: a call edge points at a type: {edges:?}"
        );
    }
}

#[test]
fn calls_only_ever_target_something_callable() {
    // The invariant behind the functional-cast rule, asserted rather than
    // left as a property of the fixture.
    let ir = analyze(&fixture());

    for suffix in ["render.c", "draw.cpp"] {
        let miscast: Vec<(RelationshipKind, String, u32)> = call_edges(&ir, suffix)
            .into_iter()
            .filter(|(kind, to, _)| {
                *kind == RelationshipKind::Calls
                    && !(to.ends_with("::function") || to.ends_with("::method"))
            })
            .collect();

        assert!(
            miscast.is_empty(),
            "{suffix}: call edges pointing at something that cannot be called: {miscast:?}"
        );
    }
}

#[test]
fn call_edges_reach_a_user_stamped_resolved() {
    // Through `dispatch`, where the Tier 1 stamp is applied — the only layer
    // at which this property is real. A new edge kind that missed it would
    // silently turn every C repository into one with structural regions and
    // suppress the safety verdict.
    use graphyn_core::ir::Resolution;

    let root = fixture();
    let ir = graphyn_lang::dispatch::analyze_files(&root, &all_files(&root))
        .expect("dispatch analysis must succeed");

    let calls: Vec<(&str, u32, &Resolution)> = ir
        .files
        .iter()
        .flat_map(|f| f.relationships.iter())
        .filter(|r| r.kind == RelationshipKind::Calls || r.kind == RelationshipKind::Instantiates)
        .map(|r| (r.file.as_str(), r.line, &r.resolution))
        .collect();

    assert!(
        !calls.is_empty(),
        "no call-like edge survived dispatch at all"
    );
    assert!(
        calls.iter().all(|(_, _, res)| **res == Resolution::Resolved),
        "call edges left at the structural default: {calls:?}"
    );
}
