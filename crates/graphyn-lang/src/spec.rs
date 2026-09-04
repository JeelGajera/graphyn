//! What Graphyn knows about a language, and how much it knows.
//!
//! # Why tiers exist
//!
//! Consolidating the adapters into one crate removed the *packaging* cost of a
//! language. It did not remove the engineering cost, and pretending otherwise
//! would make the gates in later phases unsafe.
//!
//! Parsing is tree-sitter and is a solved, vendored problem. Resolution is not:
//! every bug 0.2.0 fixed was a resolution bug — property access compared
//! against a hardcoded receiver name, `#include` producing zero edges, Go
//! imports pointing at an arbitrary package member, Rust use-paths matching
//! leaf names repository-wide. Import resolution, alias resolution and
//! declared-type binding are irreducibly per-language and cost weeks, not
//! hours.
//!
//! So "supports thirty languages" has to mean something precise, or it is
//! dishonest:
//!
//! | Tier | What it gives you | Safe to gate on |
//! |---|---|---|
//! | 1 — Resolved | Full import, alias and declared-type resolution | Yes |
//! | 2 — Structural | Symbols and intra-file references only | No — advisory |
//!
//! A Tier 2 language is a low-confidence region *by construction*. That is what
//! makes broad coverage safe rather than reckless: a gate can fail open on it
//! automatically instead of reporting a pass it did not earn.
//!
//! # Why Tier 2 costs almost nothing
//!
//! Every tree-sitter grammar Graphyn vendors ships a `queries/tags.scm` and
//! exposes it as `TAGS_QUERY`, using a small standard capture vocabulary:
//! `@definition.{class,function,method,type,interface,module,constant,macro}`,
//! `@reference.{call,type,class,implementation}`, and `@name`. That vocabulary
//! maps onto Graphyn's own `SymbolKind` and `RelationshipKind` directly.
//!
//! So a Tier 2 language does not need query files written for it at all — see
//! [`crate::structural`], which is one generic implementation driven by
//! whatever `tags.scm` the grammar already ships. Adding one is a dependency,
//! a feature flag, and a spec.

use std::path::{Path, PathBuf};

use graphyn_core::ir::{FileIR, Language};

/// How much Graphyn can resolve in a language.
///
/// Ordered so that comparisons read naturally: `Structural < Resolved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// Symbols and intra-file references. No cross-file import resolution, no
    /// alias tracking, no declared-type binding. Advisory only.
    Structural,
    /// Full import, alias and declared-type resolution.
    Resolved,
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Tier::Structural => "structural",
            Tier::Resolved => "resolved",
        }
    }

    /// The tier number as the README and `status` report it.
    pub fn number(&self) -> u8 {
        match self {
            Tier::Resolved => 1,
            Tier::Structural => 2,
        }
    }

    /// Whether a gate may draw a conclusion from this region.
    ///
    /// Tier 2 cannot see across files, so "nothing references this" is a
    /// statement about one file rather than about the repository. A gate that
    /// treated it otherwise would report a pass it had not earned.
    pub fn is_gate_safe(&self) -> bool {
        matches!(self, Tier::Resolved)
    }
}

/// One language Graphyn can analyse.
pub trait LanguageSpec: Send + Sync {
    fn language(&self) -> Language;
    fn tier(&self) -> Tier;

    /// The name reported to users.
    fn name(&self) -> &'static str;

    /// File extensions this spec claims, without the dot.
    fn extensions(&self) -> &'static [&'static str];

    /// The grammar, for the structural analyzer.
    ///
    /// Tier 1 languages parse through their own pipeline and never call this,
    /// so it is optional rather than required of every spec.
    fn grammar(&self) -> Option<tree_sitter::Language> {
        None
    }

    /// The grammar's own tags query, which drives [`crate::structural`].
    ///
    /// Upstream grammars ship one; there is no need to write it.
    fn tags_query(&self) -> Option<&'static str> {
        None
    }

    /// A Tier 1 language's own pipeline.
    ///
    /// Returning `None` opts into the structural default. The five Tier 1
    /// languages keep the pipelines they already had rather than being
    /// rewritten into resolution hooks: they work, they are tested, and a
    /// rewrite would forfeit the one property this change has to preserve.
    fn analyze(&self, _root: &Path, _files: &[PathBuf]) -> Option<Result<Vec<FileIR>, String>> {
        None
    }
}

/// Every language this build carries, in a fixed order.
///
/// Ordered rather than hash-ordered because it reaches the user through
/// `status` and `--help`, and Graphyn's first guarantee is that identical
/// input produces identical output.
pub fn specs() -> Vec<&'static dyn LanguageSpec> {
    let out: [&'static dyn LanguageSpec; NUM_SPECS] = [
        #[cfg(feature = "typescript")]
        &crate::lang::typescript::Spec,
        #[cfg(feature = "python")]
        &crate::lang::python::Spec,
        #[cfg(feature = "rust")]
        &crate::lang::rust::Spec,
        #[cfg(feature = "go")]
        &crate::lang::go::Spec,
        #[cfg(feature = "c")]
        &crate::lang::c::Spec,
        #[cfg(feature = "java")]
        &crate::lang::java::Spec,
    ];
    out.to_vec()
}

/// How many specs this build carries, so the array above has a length.
const NUM_SPECS: usize = cfg!(feature = "typescript") as usize
    + cfg!(feature = "python") as usize
    + cfg!(feature = "rust") as usize
    + cfg!(feature = "go") as usize
    + cfg!(feature = "c") as usize
    + cfg!(feature = "java") as usize;

/// The spec that owns `language`, if this build carries it.
pub fn for_language(language: &Language) -> Option<&'static dyn LanguageSpec> {
    specs().into_iter().find(|s| s.language() == *language)
}

/// A language and how much of it Graphyn can resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSupport {
    pub name: &'static str,
    pub tier: Tier,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_one_is_the_only_gate_safe_tier() {
        // The whole point of the distinction. If this ever becomes true for
        // Structural, every gate in Phases 2 and 5 silently starts trusting
        // intra-file-only data.
        assert!(Tier::Resolved.is_gate_safe());
        assert!(!Tier::Structural.is_gate_safe());
        assert_eq!(Tier::Resolved.number(), 1);
        assert_eq!(Tier::Structural.number(), 2);
    }

    #[test]
    fn every_spec_claims_at_least_one_extension() {
        for spec in specs() {
            assert!(
                !spec.extensions().is_empty(),
                "{} claims no extensions, so no file would ever reach it",
                spec.name()
            );
        }
    }

    #[test]
    fn no_two_specs_claim_the_same_extension() {
        // Two specs claiming one extension would make routing depend on
        // registry order rather than on the language.
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for spec in specs() {
            for ext in spec.extensions() {
                if let Some((other, _)) = seen.iter().find(|(_, e)| e == ext) {
                    panic!("both {other} and {} claim .{ext}", spec.name());
                }
                seen.push((spec.name(), ext));
            }
        }
    }

    #[test]
    fn a_structural_spec_can_actually_run() {
        // A Tier 2 spec with no grammar or no tags query would register as
        // supported and then analyse nothing.
        for spec in specs().into_iter().filter(|s| s.tier() == Tier::Structural) {
            assert!(
                spec.grammar().is_some(),
                "{} is Tier 2 but has no grammar",
                spec.name()
            );
            assert!(
                spec.tags_query().is_some(),
                "{} is Tier 2 but has no tags query",
                spec.name()
            );
        }
    }
}
