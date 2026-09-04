//! Java — the first Tier 2 language.
//!
//! Deliberately the whole module. There is no parser, no extractor and no
//! resolver here, because a Tier 2 language needs none: analysis runs through
//! [`crate::structural`], driven by the `tags.scm` the grammar already ships.
//!
//! This is the demonstration that adding a language is a dependency, a feature
//! flag and a spec. What it is *not* is a claim that Graphyn understands Java
//! the way it understands the Tier 1 languages: nothing here resolves an
//! import, follows an alias, or binds a declared type, and `graphyn status`
//! says so.
//!
//! Promoting Java to Tier 1 means implementing import resolution, alias
//! resolution and declared-type binding for it. That is weeks of work and is
//! explicitly out of scope for 1.0.0.

use graphyn_core::ir::Language;

use crate::spec::{LanguageSpec, Tier};

pub struct Spec;

impl LanguageSpec for Spec {
    fn language(&self) -> Language {
        Language::Java
    }

    fn tier(&self) -> Tier {
        Tier::Structural
    }

    fn name(&self) -> &'static str {
        "Java"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn grammar(&self) -> Option<tree_sitter::Language> {
        Some(tree_sitter_java::language())
    }

    fn tags_query(&self) -> Option<&'static str> {
        // The grammar's own query, not one written here. It captures class,
        // interface and method definitions, plus calls, superclasses and
        // implemented types.
        Some(tree_sitter_java::TAGS_QUERY)
    }
}
