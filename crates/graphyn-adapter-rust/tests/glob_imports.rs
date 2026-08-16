mod common;

use common::*;
use graphyn_core::ir::RelationshipKind;

#[test]
fn glob_imports_resolve_to_the_module_they_name() {
    let repo = analyze("language_features");
    let reporting = file(&repo, "services/reporting.rs");

    // `use crate::models::*` is a dependency on the module as a whole; there is
    // no single symbol to point at.
    assert!(
        has_edge(
            reporting,
            RelationshipKind::Imports,
            "models/mod.rs::module::module"
        ),
        "a glob import should resolve to the module's own symbol, have: {:?}",
        reporting
            .relationships
            .iter()
            .map(|r| r.to.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn glob_imports_do_not_invent_an_external_package() {
    let repo = analyze("language_features");
    let reporting = file(&repo, "services/reporting.rs");

    assert!(
        !has_edge(reporting, RelationshipKind::Imports, "ext::crate::package"),
        "a local glob import must not be reported as a third-party dependency"
    );
}
